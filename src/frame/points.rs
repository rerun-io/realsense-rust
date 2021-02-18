//! Type for representing a Points frame.
//!
//! A Points frame is a RealSense point cloud storage class.

use super::prelude::{CouldNotGetFrameSensorError, FrameConstructionError, FrameEx};
use crate::{
    check_rs2_error,
    common::*,
    kind::{Extension, Rs2Extension, Rs2FrameMetadata, Rs2TimestampDomain},
    sensor::Sensor,
    stream::StreamProfile,
};
use anyhow::Result;
use num_traits::ToPrimitive;
use std::convert::TryFrom;

/// Holds the raw data pointer and derived data for an RS2 Points frame.
///
/// All fields in this struct are initialized during struct creation (via `try_from`).
/// Everything called from here during runtime should be valid as long as the
/// Frame is in scope... like normal Rust.
pub struct PointsFrame<'a> {
    /// The raw data pointer from the original rs2 frame.
    frame_ptr: NonNull<sys::rs2_frame>,
    /// The timestamp of the frame.
    timestamp: f64,
    /// The RealSense time domain from which the timestamp is derived.
    timestamp_domain: Rs2TimestampDomain,
    /// The Stream Profile that created the frame.
    frame_stream_profile: StreamProfile<'a>,
    /// The number of points represented in the Points frame.
    num_points: usize,
    /// The raw pointer to the vertex data.
    vertices_data_ptr: NonNull<sys::rs2_vertex>,
    /// The raw pointer to the texture data.
    texture_data_ptr: NonNull<sys::rs2_pixel>,
    /// A boolean used during `Drop` calls. This allows for proper handling of the pointer
    /// during ownership transfer.
    should_drop: bool,
}

impl<'a> Extension for PointsFrame<'a> {
    fn extension() -> Rs2Extension {
        Rs2Extension::Points
    }
}

impl<'a> FrameEx<'a> for PointsFrame<'a> {
    fn profile(&'a self) -> &'a StreamProfile<'a> {
        &self.frame_stream_profile
    }

    fn sensor(&self) -> Result<Sensor> {
        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();
            let sensor_ptr = sys::rs2_get_frame_sensor(self.frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, CouldNotGetFrameSensorError)?;

            Ok(Sensor::try_from(NonNull::new(sensor_ptr).unwrap())?)
        }
    }

    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    fn timestamp_domain(&self) -> Rs2TimestampDomain {
        self.timestamp_domain
    }

    fn metadata(&self, metadata_kind: Rs2FrameMetadata) -> Option<std::os::raw::c_longlong> {
        if !self.supports_metadata(metadata_kind) {
            return None;
        }

        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();

            let val = sys::rs2_get_frame_metadata(
                self.frame_ptr.as_ptr(),
                metadata_kind.to_u32().unwrap(),
                &mut err,
            );
            err.as_ref()?;

            Some(val)
        }
    }

    fn supports_metadata(&self, metadata_kind: Rs2FrameMetadata) -> bool {
        unsafe {
            let mut err = std::ptr::null_mut::<sys::rs2_error>();

            let supports_metadata = sys::rs2_supports_frame_metadata(
                self.frame_ptr.as_ptr(),
                metadata_kind.to_u32().unwrap(),
                &mut err,
            );

            err.as_ref().is_none() && supports_metadata != 0
        }
    }

    unsafe fn get_owned_frame_ptr(mut self) -> NonNull<sys::rs2_frame> {
        self.should_drop = false;

        self.frame_ptr
    }
}

impl<'a> Drop for PointsFrame<'a> {
    /// Drop the raw pointer stored with this struct whenever it goes out of scope.
    fn drop(&mut self) {
        unsafe {
            if self.should_drop {
                // Note: Vertices and Texture pointer lifetimes are managed by the
                // frame itself, so dropping the frame should suffice.
                sys::rs2_release_frame(self.frame_ptr.as_ptr());
            }
        }
    }
}

unsafe impl<'a> Send for PointsFrame<'a> {}

impl<'a> std::convert::TryFrom<NonNull<sys::rs2_frame>> for PointsFrame<'a> {
    type Error = anyhow::Error;

    /// Attempt to construct a points frame from a raw pointer to `rs2_frame`
    ///
    /// All members of the `PointsFrame` struct are validated and populated during this call.
    ///
    /// # Errors
    ///
    /// There are a number of errors that may occur if the data in the `rs2_frame` is not valid, all
    /// of type [`FrameConstructionError`].
    ///
    /// - [`CouldNotGetTimestamp`](FrameConstructionError::CouldNotGetTimestamp)
    /// - [`CouldNotGetTimestampDomain`](FrameConstructionError::CouldNotGetTimestampDomain)
    /// - [`CouldNotGetFrameStreamProfile`](FrameConstructionError::CouldNotGetFrameStreamProfile)
    /// - [`CouldNotGetPointCount`](FrameConstructionError::CouldNotGetPointCount)
    /// - [`CouldNotGetData`](FrameConstructionError::CouldNotGetData)
    ///
    /// See [`FrameConstructionError`] documentation for more details.
    ///
    fn try_from(frame_ptr: NonNull<sys::rs2_frame>) -> Result<Self, Self::Error> {
        unsafe {
            let mut err = ptr::null_mut::<sys::rs2_error>();

            let timestamp = sys::rs2_get_frame_timestamp(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetTimestamp)?;

            let timestamp_domain =
                sys::rs2_get_frame_timestamp_domain(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetTimestampDomain)?;

            let profile_ptr = sys::rs2_get_frame_stream_profile(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetFrameStreamProfile)?;

            let nonnull_profile_ptr =
                NonNull::new(profile_ptr as *mut sys::rs2_stream_profile).unwrap();
            let profile = StreamProfile::try_from(nonnull_profile_ptr)?;

            let num_points = sys::rs2_get_frame_points_count(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetPointCount)?;

            let vertices_ptr = sys::rs2_get_frame_vertices(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetData)?;

            let texture_ptr = sys::rs2_get_frame_texture_coordinates(frame_ptr.as_ptr(), &mut err);
            check_rs2_error!(err, FrameConstructionError::CouldNotGetData)?;

            Ok(PointsFrame {
                frame_ptr,
                timestamp,
                timestamp_domain: Rs2TimestampDomain::from_u32(timestamp_domain).unwrap(),
                frame_stream_profile: profile,
                num_points: num_points as usize,
                vertices_data_ptr: NonNull::new(vertices_ptr).unwrap(),
                texture_data_ptr: NonNull::new(texture_ptr).unwrap(),
                should_drop: true,
            })
        }
    }
}

impl<'a> PointsFrame<'a> {
    /// Gets vertices of the point cloud.
    pub fn vertices(&'a self) -> &'a [sys::rs2_vertex] {
        unsafe {
            slice::from_raw_parts::<sys::rs2_vertex>(
                self.vertices_data_ptr.as_ptr(),
                self.num_points,
            )
        }
    }

    /// Retrieve the texture coordinates (uv map) for the point cloud.
    ///
    /// # Safety
    ///
    /// The librealsense2 C++ API directly casts the rs2_pixel* returned from
    /// rs2_get_frame_texture_coordinates() into a texture_coordinate*, thereby
    /// re-interpreting [[c_int; 2]; N] as [[c_float; 2]; N] values.
    /// Note that C does not generally guarantee that sizeof(int) == sizeof(float).
    ///
    pub fn texture_coordinates(&'a self) -> &'a [[f32; 2]] {
        unsafe {
            slice::from_raw_parts::<[f32; 2]>(
                self.texture_data_ptr.as_ptr().cast::<[f32; 2]>(),
                self.num_points,
            )
        }
    }

    /// Gets number of points in the point cloud.
    pub fn points_count(&self) -> usize {
        self.num_points
    }
}
