#![allow(deprecated)]
use super::{asbd_from_config, check_os_status, frames_to_duration, host_time_to_stream_instant};

use super::OSStatus;
use crate::host::coreaudio::macos::loopback::LoopbackDevice;
use crate::traits::{HostTrait, StreamTrait};
use crate::{BackendSpecificError, DevicesError, PauseStreamError, PlayStreamError};
use coreaudio::audio_unit::AudioUnit;
use objc2_core_audio::AudioDeviceID;
use std::sync::{mpsc, Arc, Mutex, Weak};

pub use self::enumerate::{default_input_device, default_output_device, Devices};

use objc2_core_audio::{
    kAudioDevicePropertyDeviceIsAlive, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID,
    AudioDeviceStart, AudioDeviceStop, AudioObjectPropertyAddress,
};
use property_listener::AudioObjectPropertyListener;

mod device;
pub mod enumerate;
mod loopback;
mod property_listener;
pub use device::Device;

/// Coreaudio host, the default host on macOS.
#[derive(Debug)]
pub struct Host;

impl Host {
    pub fn new() -> Result<Self, crate::HostUnavailable> {
        Ok(Host)
    }
}

impl HostTrait for Host {
    type Devices = Devices;
    type Device = Device;

    fn is_available() -> bool {
        // Assume coreaudio is always available
        true
    }

    fn devices(&self) -> Result<Self::Devices, DevicesError> {
        Devices::new()
    }

    fn default_input_device(&self) -> Option<Self::Device> {
        default_input_device()
    }

    fn default_output_device(&self) -> Option<Self::Device> {
        default_output_device()
    }
}

/// Type alias for the error callback to reduce complexity
type ErrorCallback = Box<dyn FnMut(crate::StreamError) + Send + 'static>;

/// Invoke error callback, recovering from poisoned mutex if needed.
/// Returns true if callback was invoked, false if skipped due to WouldBlock.
#[inline]
fn invoke_error_callback<E>(error_callback: &Arc<Mutex<E>>, err: crate::StreamError) -> bool
where
    E: FnMut(crate::StreamError) + Send,
{
    match error_callback.try_lock() {
        Ok(mut cb) => {
            cb(err);
            true
        }
        Err(std::sync::TryLockError::Poisoned(guard)) => {
            // Recover from poisoned lock to still report this error
            guard.into_inner()(err);
            true
        }
        Err(std::sync::TryLockError::WouldBlock) => {
            // Skip if callback is busy
            false
        }
    }
}

/// Manages device disconnection listener on a dedicated thread to ensure the
/// AudioObjectPropertyListener is always created and dropped on the same thread.
/// This avoids potential threading issues with CoreAudio APIs.
///
/// When a device disconnects, this manager:
/// 1. Attempts to pause the stream to stop audio I/O
/// 2. Calls the error callback with `StreamError::DeviceNotAvailable`
///
/// The dedicated thread architecture ensures `Stream` can implement `Send`.
struct DisconnectManager {
    _shutdown_tx: mpsc::Sender<()>,
}

impl DisconnectManager {
    /// Create a new DisconnectManager that monitors device disconnection on a dedicated thread
    fn new(
        device_id: AudioDeviceID,
        stream_weak: Weak<Mutex<StreamInner>>,
        error_callback: Arc<Mutex<ErrorCallback>>,
    ) -> Result<Self, crate::BuildStreamError> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let (disconnect_tx, disconnect_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        // Spawn dedicated thread to own the AudioObjectPropertyListener
        let disconnect_tx_clone = disconnect_tx.clone();
        std::thread::spawn(move || {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyDeviceIsAlive,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            // Create the listener on this dedicated thread
            match AudioObjectPropertyListener::new(device_id, property_address, move || {
                let _ = disconnect_tx_clone.send(());
            }) {
                Ok(_listener) => {
                    let _ = ready_tx.send(Ok(()));
                    // Drop the listener on this thread after receiving a shutdown signal
                    let _ = shutdown_rx.recv();
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            }
        });

        // Wait for listener creation to complete or fail
        ready_rx
            .recv()
            .map_err(|_| crate::BuildStreamError::BackendSpecific {
                err: BackendSpecificError {
                    description: "Disconnect listener thread terminated unexpectedly".to_string(),
                },
            })??;

        // Handle disconnect events on the main thread pool
        let stream_weak_clone = stream_weak.clone();
        let error_callback_clone = error_callback.clone();
        std::thread::spawn(move || {
            while disconnect_rx.recv().is_ok() {
                // Check if stream still exists
                if let Some(stream_arc) = stream_weak_clone.upgrade() {
                    // First, try to pause the stream to stop playback
                    if let Ok(mut stream_inner) = stream_arc.try_lock() {
                        let _ = stream_inner.pause();
                    }

                    // Always try to notify about device disconnection
                    invoke_error_callback(
                        &error_callback_clone,
                        crate::StreamError::DeviceNotAvailable,
                    );
                } else {
                    // Stream is gone, exit the handler thread
                    break;
                }
            }
        });

        Ok(DisconnectManager {
            _shutdown_tx: shutdown_tx,
        })
    }
}

/// CHROMATICITY PATCH: controls a direct Core Audio device IOProc — the macOS
/// system-audio loopback read path that replaces cpal's AUHAL `AudioUnit` (see
/// `third_party/cpal/UPSTREAM-VERSION.md` and `docs/adr/ADR-022` Amendment
/// 2026-05..06-03). The `AudioDeviceIOProcID` is a fn-pointer-shaped handle, so
/// this struct is `Send`; the realtime block holds its own captured state,
/// which Core Audio `Block_copy`s at creation and releases at
/// `AudioDeviceDestroyIOProcID`.
struct IoProcController {
    device_id: AudioDeviceID,
    io_proc_id: AudioDeviceIOProcID,
}

impl IoProcController {
    fn start(&self) -> Result<(), PlayStreamError> {
        check_os_status(unsafe { AudioDeviceStart(self.device_id, self.io_proc_id) })
            .map_err(Into::into)
    }

    fn stop(&self) -> Result<(), PauseStreamError> {
        check_os_status(unsafe { AudioDeviceStop(self.device_id, self.io_proc_id) })
            .map_err(Into::into)
    }
}

impl Drop for IoProcController {
    fn drop(&mut self) {
        // Stop + uninstall the IOProc BEFORE the aggregate device (and tap) are
        // destroyed by `_loopback_device`'s Drop. Field order in `StreamInner`
        // (this controller declared before `_loopback_device`) guarantees that.
        // Status is unchecked to avoid panicking during drop.
        unsafe {
            let _ = AudioDeviceStop(self.device_id, self.io_proc_id);
            let _ = AudioDeviceDestroyIOProcID(self.device_id, self.io_proc_id);
        }
    }
}

struct StreamInner {
    playing: bool,
    /// `Some` for the AUHAL paths (microphone input + output streams); `None`
    /// for the direct-IOProc system-audio loopback path.
    audio_unit: Option<AudioUnit>,
    /// CHROMATICITY PATCH: `Some` only for the direct-IOProc loopback path.
    /// Declared BEFORE `_loopback_device` so it drops first — stopping +
    /// destroying the IOProc before its aggregate device + tap are torn down.
    io_proc: Option<IoProcController>,
    // Track the device with which the stream was spawned.
    //
    // We must do this so that we can avoid changing the device sample rate if there is already
    // a stream associated with the device.
    #[allow(dead_code)]
    device_id: AudioDeviceID,
    /// Manage the lifetime of the aggregate device used
    /// for loopback recording
    _loopback_device: Option<LoopbackDevice>,
}

impl StreamInner {
    fn play(&mut self) -> Result<(), PlayStreamError> {
        if !self.playing {
            if let Some(audio_unit) = self.audio_unit.as_mut() {
                if let Err(e) = audio_unit.start() {
                    let description = format!("{e}");
                    let err = BackendSpecificError { description };
                    return Err(err.into());
                }
            } else if let Some(io_proc) = self.io_proc.as_ref() {
                io_proc.start()?;
            }
            self.playing = true;
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), PauseStreamError> {
        if self.playing {
            if let Some(audio_unit) = self.audio_unit.as_mut() {
                if let Err(e) = audio_unit.stop() {
                    let description = format!("{e}");
                    let err = BackendSpecificError { description };
                    return Err(err.into());
                }
            } else if let Some(io_proc) = self.io_proc.as_ref() {
                io_proc.stop()?;
            }
            self.playing = false;
        }
        Ok(())
    }
}

impl Drop for StreamInner {
    fn drop(&mut self) {
        // CHROMATICITY PATCH: enforce the loopback teardown order EXPLICITLY
        // rather than relying on field-declaration drop order — stop + destroy
        // the IOProc (`IoProcController::drop`) BEFORE the aggregate device + tap
        // that `_loopback_device` owns (`LoopbackDevice::drop`). Reorder-proof
        // belt-and-suspenders for this load-bearing unsafe teardown. The AUHAL
        // paths leave both fields `None`, so these are no-ops there.
        self.io_proc = None;
        self._loopback_device = None;
    }
}

pub struct Stream {
    inner: Arc<Mutex<StreamInner>>,
    // Manages the device disconnection listener separately to allow Stream to be Send.
    // The DisconnectManager contains the non-Send AudioObjectPropertyListener.
    _disconnect_manager: DisconnectManager,
}

impl Stream {
    fn new(
        inner: StreamInner,
        error_callback: ErrorCallback,
    ) -> Result<Self, crate::BuildStreamError> {
        let device_id = inner.device_id;
        let inner_arc = Arc::new(Mutex::new(inner));
        let weak_inner = Arc::downgrade(&inner_arc);

        let error_callback = Arc::new(Mutex::new(error_callback));
        let disconnect_manager = DisconnectManager::new(device_id, weak_inner, error_callback)?;

        Ok(Self {
            inner: inner_arc,
            _disconnect_manager: disconnect_manager,
        })
    }
}

impl StreamTrait for Stream {
    fn play(&self) -> Result<(), PlayStreamError> {
        let mut stream = self
            .inner
            .lock()
            .map_err(|_| PlayStreamError::BackendSpecific {
                err: BackendSpecificError {
                    description: "A cpal stream operation panicked while holding the lock - this is a bug, please report it".to_string(),
                },
            })?;

        stream.play()
    }

    fn pause(&self) -> Result<(), PauseStreamError> {
        let mut stream = self
            .inner
            .lock()
            .map_err(|_| PauseStreamError::BackendSpecific {
                err: BackendSpecificError {
                    description: "A cpal stream operation panicked while holding the lock - this is a bug, please report it".to_string(),
                },
            })?;

        stream.pause()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        default_host,
        traits::{DeviceTrait, HostTrait, StreamTrait},
        Sample,
    };

    #[test]
    fn test_play() {
        let host = default_host();
        let device = host.default_output_device().unwrap();

        let mut supported_configs_range = device.supported_output_configs().unwrap();
        let supported_config = supported_configs_range
            .next()
            .unwrap()
            .with_max_sample_rate();
        let config = supported_config.config();

        let stream = device
            .build_output_stream(
                &config,
                write_silence::<f32>,
                move |err| println!("Error: {err}"),
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();
        stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    #[test]
    fn test_record() {
        let host = default_host();
        let device = host.default_input_device().unwrap();
        println!("Device: {:?}", device.name());

        let mut supported_configs_range = device.supported_input_configs().unwrap();
        println!("Supported configs:");
        for config in supported_configs_range.clone() {
            println!("{:?}", config)
        }
        let supported_config = supported_configs_range
            .next()
            .unwrap()
            .with_max_sample_rate();
        let config = supported_config.config();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &crate::InputCallbackInfo| {
                    // react to stream events and read or write stream data here.
                    println!("Got data: {:?}", &data[..25]);
                },
                move |err| println!("Error: {err}"),
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();
        stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    #[test]
    fn test_record_output() {
        if std::env::var("CI").is_ok() {
            println!("Skipping test_record_output in CI environment due to permissions");
            return;
        }

        let host = default_host();
        let device = host.default_output_device().unwrap();

        let mut supported_configs_range = device.supported_output_configs().unwrap();
        let supported_config = supported_configs_range
            .next()
            .unwrap()
            .with_max_sample_rate();
        let config = supported_config.config();

        println!("Building input stream");
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &crate::InputCallbackInfo| {
                    // react to stream events and read or write stream data here.
                    println!("Got data: {:?}", &data[..25]);
                },
                move |err| println!("Error: {err}"),
                None, // None=blocking, Some(Duration)=timeout
            )
            .unwrap();
        stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    fn write_silence<T: Sample>(data: &mut [T], _: &crate::OutputCallbackInfo) {
        for sample in data.iter_mut() {
            *sample = Sample::EQUILIBRIUM;
        }
    }
}
