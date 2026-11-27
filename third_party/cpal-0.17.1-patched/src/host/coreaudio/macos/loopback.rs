//! Manages loopback recording (recording system audio output)

use super::device::Device;
use crate::{BuildStreamError, host::coreaudio::check_os_status};
use objc2::{AnyThread, rc::Retained};
use objc2_core_audio::{
    AudioHardwareCreateAggregateDevice, AudioHardwareCreateProcessTap,
    AudioHardwareDestroyAggregateDevice, AudioHardwareDestroyProcessTap, AudioObjectID,
    CATapDescription, CATapMuteBehavior, kAudioAggregateDeviceIsPrivateKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceTapAutoStartKey,
    kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioSubTapDriftCompensationKey,
    kAudioSubTapUIDKey,
};
use objc2_core_foundation::{
    CFArray, CFDictionary, CFMutableDictionary, CFRetained, CFString, CFStringCreateWithCString,
    kCFAllocatorDefault, kCFTypeArrayCallBacks, kCFTypeDictionaryKeyCallBacks,
    kCFTypeDictionaryValueCallBacks,
};
use objc2_foundation::{NSArray, NSNumber, NSString, ns_string};
use std::{
    ffi::{CStr, c_void},
    ptr::NonNull,
};

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
    /// Create a [`LoopbackDevice`] that records system audio output.
    pub fn from_device(_device: &Device) -> Result<Self, BuildStreamError> {
        // 1 - Create tap
        // Empty exclusion list so we capture all processes' system audio output.
        let processes: Retained<NSArray<NSNumber>> = NSArray::new();
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

        let mut tap_obj_id: AudioObjectID = 0;
        let status =
            unsafe { AudioHardwareCreateProcessTap(Some(tap_desc.as_ref()), &mut tap_obj_id) };
        check_os_status(status)?;
        let tap_uid = unsafe { tap_desc.UUID().UUIDString() };

        // 2 - Create aggregate device
        let aggregate_device_properties = create_audio_aggregate_device_properties(tap_uid);
        let mut aggregate_device_id: AudioObjectID = 0;
        let status = unsafe {
            AudioHardwareCreateAggregateDevice(
                aggregate_device_properties.as_ref(),
                NonNull::from(&mut aggregate_device_id),
            )
        };
        if let Err(err) = check_os_status(status) {
            unsafe {
                let _ = AudioHardwareDestroyProcessTap(tap_obj_id);
            }
            return Err(err.into());
        }

        Ok(Self {
            tap_id: tap_obj_id,
            aggregate_device: Device::new(aggregate_device_id),
        })
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
        // Unique per-instance aggregate device UID so concurrent or restarted
        // loopback streams never collide in Core Audio.
        let aggregate_uid =
            CFString::from_str(&format!("com.cpal.LoopbackRecordAggregateDevice.{tap_uid}"));
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceUIDKey) as *const _ as *const c_void,
            &*aggregate_uid as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceTapListKey) as *const _ as *const c_void,
            &*taps as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceTapAutoStartKey) as *const _ as *const c_void,
            &*NSNumber::initWithBool(NSNumber::alloc(), true) as *const _ as *const c_void,
        );
        CFMutableDictionary::set_value(
            Some(dict.as_ref()),
            &*to_cfstring(kAudioAggregateDeviceIsPrivateKey) as *const _ as *const c_void,
            &*NSNumber::initWithBool(NSNumber::alloc(), true) as *const _ as *const c_void,
        );

        CFRetained::cast_unchecked::<CFDictionary>(dict)
    };

    aggregate_dev_properties
}
