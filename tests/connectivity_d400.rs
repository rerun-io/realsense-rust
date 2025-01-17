//! Tests for evaluating connectivity / configuration of sensors

#![cfg(feature = "test-single-device")]

use realsense_rust::{
    base::Rs2Roi,
    config::Config,
    context::Context,
    frame::{ColorFrame, DepthFrame, FrameEx, InfraredFrame},
    kind::{Rs2CameraInfo, Rs2Extension, Rs2Format, Rs2Option, Rs2ProductLine, Rs2StreamKind},
    pipeline::InactivePipeline,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    time::Duration,
};

#[test]
fn d400_can_resolve_color_and_depth_and_infrared() {
    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::D400);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();

        let usb_cstr = device.info(Rs2CameraInfo::UsbTypeDescriptor).unwrap();
        let usb_val: f32 = usb_cstr.to_str().unwrap().parse().unwrap();
        if usb_val >= 3.0 {
            config
                .enable_device_from_serial(serial)
                .unwrap()
                .disable_all_streams()
                .unwrap()
                .enable_stream(Rs2StreamKind::Color, Some(0), 0, 0, Rs2Format::Rgba8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Depth, Some(0), 0, 0, Rs2Format::Z16, 30)
                .unwrap()
                // RealSense doesn't seem to like index zero for the IR cameras
                //
                // Really not sure why? This seems like an implementation issue, but in practice most
                // won't be after the IR image directly (I think?).
                .enable_stream(Rs2StreamKind::Infrared, Some(1), 0, 0, Rs2Format::Y8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Infrared, Some(2), 0, 0, Rs2Format::Any, 30)
                .unwrap();
        } else {
            config
                .enable_device_from_serial(serial)
                .unwrap()
                .disable_all_streams()
                .unwrap()
                .enable_stream(Rs2StreamKind::Color, Some(0), 0, 0, Rs2Format::Rgba8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Depth, Some(0), 0, 0, Rs2Format::Z16, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Infrared, Some(1), 0, 0, Rs2Format::Y8, 30)
                .unwrap();
        }

        let pipeline = InactivePipeline::try_from(&context).unwrap();

        assert!(pipeline.can_resolve(&config));
        assert!(pipeline.resolve(&config).is_some());
    }
}

#[test]
fn d400_streams_at_expected_framerate() {
    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::D400);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();

        let framerate = 30;

        config
            .enable_device_from_serial(serial)
            .unwrap()
            .disable_all_streams()
            .unwrap()
            .enable_stream(Rs2StreamKind::Color, None, 0, 0, Rs2Format::Rgb8, framerate)
            .unwrap()
            .enable_stream(Rs2StreamKind::Depth, None, 0, 0, Rs2Format::Z16, framerate)
            .unwrap();

        let pipeline = InactivePipeline::try_from(&context).unwrap();

        assert!(pipeline.can_resolve(&config));

        let mut pipeline = pipeline.start(Some(config)).unwrap();

        let mut nframes = 0usize;
        let number_of_seconds = 5;
        let iters = number_of_seconds * framerate;

        let begin = std::time::SystemTime::now();
        let mut first_iter_time = 0;

        for i in 0..iters {
            let frames = if i == 0 {
                // The first frames captured always seems to have a delay.
                //
                // For the D400, this is 1.5s but can probably get worse than this. Instead, we
                // choose the default timeout for the first frame.
                let frames = pipeline.wait(None).unwrap();
                first_iter_time = begin.elapsed().unwrap().as_millis();
                frames
            } else {
                pipeline.wait(Some(Duration::from_millis(50))).unwrap()
            };
            nframes += frames.count();
        }

        let elapsed_time_ms = begin.elapsed().unwrap().as_millis();
        let expected_time_ms = 1000 * (number_of_seconds as u128);

        let absdiff_from_expected = if elapsed_time_ms > expected_time_ms {
            elapsed_time_ms - expected_time_ms
        } else {
            expected_time_ms - elapsed_time_ms
        };

        assert!(
            absdiff_from_expected <= first_iter_time + 200,
            "Difference in time from expected time: {}",
            absdiff_from_expected
        );

        assert_eq!(nframes, framerate * number_of_seconds * 2);
    }
}

#[test]
fn d400_streams_are_distinct() {
    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::D400);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();

        let usb_cstr = device.info(Rs2CameraInfo::UsbTypeDescriptor).unwrap();
        let usb_val: f32 = usb_cstr.to_str().unwrap().parse().unwrap();
        let mut expected_frame_count = 4;
        if usb_val >= 3.0 {
            // Gyro / accel streams not included here because they have a different framerate
            config
                .enable_device_from_serial(serial)
                .unwrap()
                .disable_all_streams()
                .unwrap()
                .enable_stream(Rs2StreamKind::Color, None, 0, 0, Rs2Format::Rgba8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Depth, None, 0, 0, Rs2Format::Z16, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Infrared, Some(1), 0, 0, Rs2Format::Y8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Infrared, Some(2), 0, 0, Rs2Format::Y8, 30)
                .unwrap();
        } else {
            expected_frame_count = 2;
            config
                .enable_device_from_serial(serial)
                .unwrap()
                .disable_all_streams()
                .unwrap()
                .enable_stream(Rs2StreamKind::Color, None, 0, 0, Rs2Format::Rgba8, 30)
                .unwrap()
                .enable_stream(Rs2StreamKind::Depth, None, 0, 0, Rs2Format::Z16, 30)
                .unwrap();
        }

        let pipeline = InactivePipeline::try_from(&context).unwrap();
        let mut pipeline = pipeline.start(Some(config)).unwrap();

        let frames = pipeline.wait(None).unwrap();

        assert_eq!(frames.count(), expected_frame_count);
        assert_eq!(frames.frames_of_type::<ColorFrame>().len(), 1);
        assert_eq!(frames.frames_of_type::<DepthFrame>().len(), 1);
        assert_eq!(
            frames.frames_of_type::<InfraredFrame>().len(),
            expected_frame_count - 2
        );
    }
}

// Options we will attempt to set
fn possible_options_and_vals_map() -> HashMap<Rs2Option, Option<f32>> {
    let mut options_set = HashMap::<Rs2Option, Option<f32>>::new();
    options_set.insert(Rs2Option::GlobalTimeEnabled, Some(1.0));
    options_set
}

// Options we know are ignored, and their actual returned values on `get_option`
fn supported_but_ignored_options_and_vals_map() -> HashMap<Rs2Option, Option<f32>> {
    //
    // No options like this found yet!
    //
    HashMap::<Rs2Option, Option<f32>>::new()
}

/// Check for supported but ignored sensor options.
///
/// This test is a direct result of decisions made in the Intel RealSense SDK to obfuscate the behavior of a few sensor
/// options. There are a few Options that are registered as "supported" by the sensor, but are actually just set to
/// their default values on runtime. These options are listed in `supported_but_ignored_options_and_vals_map()` above.
///
/// Currently, [Rs2Option::GlobalTimeEnabled] on the L500 is the only setting known to suffer from this. However, this
/// test has been written in a way that makes it easy to test more Options for this same behavior.
#[test]
fn d400_streams_check_supported_but_ignored_sensor_options() {
    let options_to_set = possible_options_and_vals_map();
    let options_ignored = supported_but_ignored_options_and_vals_map();

    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::L500);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        // Grab the sensor list
        for mut sensor in device.sensors() {
            for (option, val) in &options_to_set {
                // We unwrap here because we don't care about the result of the set for this test. RealSense is pretty
                // tricky when it comes to what can be set and what can't; the best way to check this would be to use
                // `sensor.supports_option` or `sensor.is_option_read_only`.
                //
                // However, there are exceptions, as one can see from setting GlobalTimeEnabled on the L500 series.
                sensor.set_option(*option, val.unwrap()).unwrap();
            }
        }
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();
        config
            .enable_device_from_serial(serial)
            .unwrap()
            .disable_all_streams()
            .unwrap()
            .enable_stream(Rs2StreamKind::Color, None, 0, 0, Rs2Format::Yuyv, 30)
            .unwrap()
            .enable_stream(Rs2StreamKind::Depth, None, 0, 0, Rs2Format::Z16, 30)
            .unwrap()
            .enable_stream(Rs2StreamKind::Infrared, None, 0, 0, Rs2Format::Y8, 30)
            .unwrap();

        let pipeline = InactivePipeline::try_from(&context).unwrap();
        let _pipeline = pipeline.start(Some(config)).unwrap();

        for sensor in device.sensors() {
            for (option, val) in &options_to_set {
                // Check that the Options we wanted to set are
                // 1. Theoretically supported by the sensor, but
                // 2. Actually discarded when set.
                if options_ignored.contains_key(option) {
                    assert!(sensor.supports_option(*option));
                    assert_ne!(
                        sensor.get_option(*option),
                        *options_ignored.get(option).unwrap()
                    );
                }
                // If we get here, it means that the option should actually set successfully. Fail if it's not.
                else {
                    assert_eq!(sensor.get_option(*option), *val);
                }
            }
        }
    }
}

/// After the startup-phase the frame number must increase by one for each new frameset as long
/// as only one stream is active and the pipeline is queried for new framesets faster than the
/// framerate.
#[test]
fn d400_frame_numbers_increase() {
    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::D400);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();

        config
            .enable_device_from_serial(serial)
            .unwrap()
            .disable_all_streams()
            .unwrap()
            .enable_stream(Rs2StreamKind::Depth, None, 0, 0, Rs2Format::Z16, 30)
            .unwrap();

        let pipeline = InactivePipeline::try_from(&context).unwrap();
        let mut pipeline = pipeline.start(Some(config)).unwrap();

        // Startup-phase: On startup the RealSense often drops some frames. Skip those.
        for _ in 0..5 {
            let _ = pipeline.wait(None).unwrap();
        }

        let mut last_frame_number: Option<u64> = None;
        for _ in 0..5 {
            let frameset = pipeline.wait(None).unwrap();
            let depth_frames = frameset.frames_of_type::<DepthFrame>();
            let frame_number = depth_frames.first().unwrap().frame_number();
            if let Some(last_frame_number) = last_frame_number {
                assert_eq!(last_frame_number + 1, frame_number);
            }
            last_frame_number = Some(frame_number);
        }
    }
}

/// Verify that the auto exposure's region of interest can be read and written.
#[test]
fn d400_region_of_interest_accessible() {
    let context = Context::new().unwrap();

    let mut queryable_set = HashSet::new();
    queryable_set.insert(Rs2ProductLine::D400);

    let devices = context.query_devices(queryable_set);

    if let Some(device) = devices.get(0) {
        let serial = device.info(Rs2CameraInfo::SerialNumber).unwrap();
        let mut config = Config::new();

        config
            .enable_device_from_serial(serial)
            .unwrap()
            .disable_all_streams()
            .unwrap()
            .enable_stream(Rs2StreamKind::Color, None, 0, 0, Rs2Format::Rgba8, 30)
            .unwrap();

        let pipeline = InactivePipeline::try_from(&context).unwrap();
        let mut pipeline = pipeline.start(Some(config)).unwrap();

        // Wait until a frame is received to make sure the camera is properly initialized.
        let _ = pipeline.wait(None).unwrap();

        let profile = pipeline.profile();
        let intrinsics = profile.streams().first().unwrap().intrinsics().unwrap();
        let width = intrinsics.width() as i32;
        let height = intrinsics.height() as i32;

        let sensors = profile.device().sensors();
        let mut color_sensor = sensors
            .into_iter()
            .find(|sensor| sensor.extension() == Rs2Extension::ColorSensor)
            .unwrap();
        color_sensor
            .set_option(Rs2Option::EnableAutoExposure, 1.0)
            .unwrap();

        let old_roi = color_sensor.get_region_of_interest().unwrap();
        assert!(0 <= old_roi.min_x && old_roi.min_x <= old_roi.max_x && old_roi.max_x < width);
        assert!(0 <= old_roi.min_y && old_roi.min_y <= old_roi.max_x && old_roi.max_y < height);

        let roi = Rs2Roi {
            min_x: width / 8,
            min_y: height / 8,
            max_x: width * 7 / 8,
            max_y: height * 7 / 8,
        };
        color_sensor.set_region_of_interest(roi).unwrap();
    }
}
