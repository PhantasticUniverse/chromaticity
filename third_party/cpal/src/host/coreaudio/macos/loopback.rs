//! Manages loopback recording (recording system audio output)

use super::device::Device;
use crate::{host::coreaudio::check_os_status, BackendSpecificError, BuildStreamError};
use objc2::{rc::Retained, AnyThread};
use objc2_core_audio::{
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceTapAutoStartKey,
    kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioDevicePropertyDeviceUID,
    kAudioEndPointDeviceIsPrivateKey, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey,
    kAudioTapPropertyFormat, AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap,
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress, CATapDescription,
    CATapMuteBehavior,
};
use objc2_core_audio_types::AudioStreamBasicDescription;
use objc2_core_foundation::{
    kCFAllocatorDefault, kCFTypeArrayCallBacks, kCFTypeDictionaryKeyCallBacks,
    kCFTypeDictionaryValueCallBacks, CFArray, CFDictionary, CFMutableDictionary, CFRetained,
    CFString, CFStringCreateWithCString,
};
use objc2_foundation::{ns_string, NSArray, NSNumber, NSString};
use std::{
    ffi::{c_void, CStr},
    mem::MaybeUninit,
    ptr::NonNull,
};
type CFStringRef = *mut std::os::raw::c_void;

impl Device {
    // CHROMATICITY PATCH: retained verbatim from upstream cpal 0.17.1 but no
    // longer called after the global-tap change in `from_device` below (see
    // third_party/cpal/UPSTREAM-VERSION.md). Kept to minimise the vendor diff.
    #[allow(dead_code)]
    fn uid(&self) -> Result<Retained<NSString>, BackendSpecificError> {
        let mut cfstring: CFStringRef = std::ptr::null_mut();
        let mut size = std::mem::size_of::<CFStringRef>() as u32;

        let property = AudioObjectPropertyAddress {
            mSelector: kAudioDevicePropertyDeviceUID,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };

        let status = unsafe {
            AudioObjectGetPropertyData(
                self.audio_device_id,
                NonNull::from(&property),
                0,
                std::ptr::null(),
                NonNull::from(&mut size),
                NonNull::from(&mut cfstring).cast(),
            )
        };
        check_os_status(status)?;

        if cfstring.is_null() {
            return Err(BackendSpecificError {
                description: "Device uid is null".to_string(),
            });
        }

        let ns_string: Retained<NSString> = unsafe {
            // unwrap cause cfstring!=null as checked before
            Retained::retain(cfstring as *mut NSString).unwrap()
        };

        Ok(ns_string)
    }
}

/// An aggregate device with tap for recording system output.
///
/// Its main difference with [`Device`] is that it's destroyed when dropped.
///
/// It also doesn't implement the [`DeviceTrait`] as users shouldn't be using it. Its
/// main purpose is to destroy the created aggregate device when loopback recording
/// is done.
#[derive(PartialEq, Eq)]
pub struct LoopbackDevice {
    pub tap_id: AudioObjectID,
    pub aggregate_device: Device,
}

impl LoopbackDevice {
    /// Create a [`LoopbackDevice`] that records the sound
    /// output of `device`.
    pub fn from_device(device: &Device) -> Result<Self, BuildStreamError> {
        // 1 - Create tap

        // CHROMATICITY PATCH (vs upstream cpal 0.17.1 — see
        // third_party/cpal/UPSTREAM-VERSION.md): build a GLOBAL stereo tap via
        // `initStereoGlobalTapButExcludeProcesses:` instead of the
        // device-scoped `initWithProcesses:andDeviceUID:withStream:`. On
        // current macOS the device-scoped initializer with an empty exclusive
        // process list creates a tap that silently delivers all-zero buffers
        // (RustAudio/cpal#329); the global initializer is the
        // AudioCap/MiniMeters-proven path that actually captures system
        // output. An empty exclude-list taps every process that outputs audio.
        // The aggregate device follows the system default output, so the
        // specific `device` is intentionally unused on this path.
        let _ = device;
        let processes = NSArray::new();
        let tap_desc = unsafe {
            CATapDescription::initStereoGlobalTapButExcludeProcesses(
                CATapDescription::alloc(),
                &processes,
            )
        };
        unsafe {
            tap_desc.setMuteBehavior(CATapMuteBehavior::Unmuted); // captured audio still goes to speakers
            tap_desc.setName(ns_string!("cpal output recorder"));
            tap_desc.setPrivate(true); // the Aggregate Device would be private
        };

        let mut tap_obj_id: MaybeUninit<AudioObjectID> = MaybeUninit::uninit();
        let tap_obj_id = unsafe {
            AudioHardwareCreateProcessTap(Some(tap_desc.as_ref()), tap_obj_id.as_mut_ptr());
            tap_obj_id.assume_init()
        };
        let tap_uid = unsafe { tap_desc.UUID().UUIDString() };

        // 2 - Create aggregate device

        // CHROMATICITY PATCH (soundness, vs upstream cpal 0.17.1): bind the
        // out-param `mut` and take `&mut` for the pointer.
        // `AudioHardwareCreateAggregateDevice` WRITES the new device id through
        // this pointer, but upstream derives it from a shared `&` of a
        // non-`mut` binding — a write through a read-only-provenance pointer is
        // UB, and the optimizer is permitted to fold the later read back to 0.
        // This matches the out-param idiom already used in this file
        // (`Device::uid`'s `&mut size` / `&mut cfstring`, and `tap_obj_id`'s
        // `MaybeUninit`). See third_party/cpal/UPSTREAM-VERSION.md.
        let aggregate_device_properties = create_audio_aggregate_device_properties(tap_uid);
        let mut aggregate_device_id: AudioObjectID = 0;
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(
                aggregate_device_properties.as_ref(),
                NonNull::from(&mut aggregate_device_id),
            )
        };
        check_os_status(status)?;

        Ok(Self {
            tap_id: tap_obj_id,
            aggregate_device: Device::new(aggregate_device_id),
        })
    }

    /// Read the tap's authoritative capture format via `kAudioTapPropertyFormat`
    /// from the TAP object (not the aggregate device).
    ///
    /// CHROMATICITY PATCH: this is the format the direct-IOProc loopback reader
    /// must honour. Deriving the format from the aggregate device's
    /// `default_input_config()` instead is what corrupted the AUHAL read path —
    /// the AUHAL's internal `AudioConverter` negotiated against the aggregate's
    /// flattened format rather than the tap's true stream description (planar
    /// Float32). See `third_party/cpal/UPSTREAM-VERSION.md` and
    /// `docs/adr/ADR-022-system-audio-capture.md` (Amendment 2026-06-03).
    pub fn tap_stream_format(&self) -> Result<AudioStreamBasicDescription, BackendSpecificError> {
        let property = AudioObjectPropertyAddress {
            mSelector: kAudioTapPropertyFormat,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        // `AudioStreamBasicDescription` is a plain C POD; zero-init then fill.
        let mut asbd: AudioStreamBasicDescription = unsafe { std::mem::zeroed() };
        let mut size = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                self.tap_id,
                NonNull::from(&property),
                0,
                std::ptr::null(),
                NonNull::from(&mut size),
                NonNull::from(&mut asbd).cast(),
            )
        };
        check_os_status(status)?;
        Ok(asbd)
    }
}

impl Drop for LoopbackDevice {
    fn drop(&mut self) {
        unsafe {
            // We don't check status to avoid panic during `drop`
            let _status =
                AudioHardwareDestroyAggregateDevice(self.aggregate_device.audio_device_id);
            let _status = AudioHardwareDestroyProcessTap(self.tap_id);
        }
    }
}

fn to_cfstring(cstr: &'static CStr) -> CFRetained<CFString> {
    unsafe {
        CFStringCreateWithCString(
            kCFAllocatorDefault,
            cstr.as_ptr(),
            0x08000100, /* UTF8 */
        )
    }
    .unwrap()
}

/// Rust reimplementation of the following:
/// ```c
/// tap_uid = [[tap_description UUID] UUIDString];
/// taps = @[
///     @{
///         @kAudioSubTapUIDKey : (NSString*)tap_uid,
///         @kAudioSubTapDriftCompensationKey : @YES,
///     },
/// ];
///
/// aggregate_device_properties = @{
///     @kAudioAggregateDeviceNameKey : @"MiniMetersAggregateDevice",
///     @kAudioAggregateDeviceUIDKey :
///         @"com.josephlyncheski.MiniMetersAggregateDevice",
///     @kAudioAggregateDeviceTapListKey : taps,
///     @kAudioAggregateDeviceTapAutoStartKey : @NO,
///     // If we set this to NO then I believe we need to make the Tap public as
///     // well.
///     @kAudioAggregateDeviceIsPrivateKey : @YES,
/// };
/// ```
pub fn create_audio_aggregate_device_properties(
    tap_uid: Retained<NSString>,
) -> CFRetained<CFDictionary> {
    let tap_inner = unsafe {
        let dict = CFMutableDictionary::new(
            kCFAllocatorDefault,
            2,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
        .unwrap();

        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioSubTapUIDKey) as *const _ as *const c_void,
            &*tap_uid as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioSubTapDriftCompensationKey) as *const _ as *const c_void,
            &*NSNumber::initWithBool(NSNumber::alloc(), true) as *const _ as *const c_void,
        );

        dict
    };
    let _taps_list = [tap_inner];
    let taps = unsafe {
        CFArray::new(
            kCFAllocatorDefault,
            _taps_list.as_ptr() as *mut *const c_void,
            _taps_list.len() as _,
            &kCFTypeArrayCallBacks,
        )
        .unwrap()
    };
    let aggregate_dev_properties = unsafe {
        let dict = CFMutableDictionary::new(
            kCFAllocatorDefault,
            5,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
        .unwrap();

        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceNameKey) as *const _ as *const c_void,
            &*CFString::from_str("Cpal loopback record aggregate device") as *const _
                as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceUIDKey) as *const _ as *const c_void,
            &*CFString::from_str("com.cpal.LoopbackRecordAggregateDevice") as *const _
                as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceTapListKey) as *const _ as *const c_void,
            &*taps as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceTapAutoStartKey) as *const _ as *const c_void,
            // CHROMATICITY PATCH (vs upstream cpal 0.17.1): auto-start the tap
            // when the aggregate device starts. With this NO, the tap never
            // begins capturing into the aggregate device, so the input stream
            // receives zero data callbacks (verified on macOS 26: `cb=0`).
            // Setting it YES is what actually makes the loopback deliver
            // buffers. See third_party/cpal/UPSTREAM-VERSION.md.
            &*NSNumber::initWithBool(NSNumber::alloc(), true) as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioEndPointDeviceIsPrivateKey) as *const _ as *const c_void,
            &*NSNumber::initWithBool(NSNumber::alloc(), true) as *const _ as *const c_void,
        );

        CFRetained::cast_unchecked::<CFDictionary>(dict)
    };

    aggregate_dev_properties
}
