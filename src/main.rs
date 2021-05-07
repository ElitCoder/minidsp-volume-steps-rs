use evdev_rs;
use lazy_static::lazy_static;
use num::clamp;
use regex::Regex;
use std::fs::File;
use std::{env, process::Command, thread, time};

// Path to control software
const MINIDSP_BINARY: &str = "minidsp";

// Factor to scale gain differences with
const GAIN_SCALE: f64 = 6.0;

// Time to sleep between calls
const SLEEP_TIME_MS: u64 = 50;

// Gain values for miniDSP 2x4 HD
const MINIDSP_GAIN_MIN: f64 = -127.0;
const MINIDSP_GAIN_MAX: f64 = 0.0;

// Gain offset to add/remove on events
const GAIN_OFFSET: f64 = 3.0;

// Lazy static needed since Regex::new() is a function call
lazy_static! {
    static ref GAIN_REGEX: Regex =
        Regex::new(r"Gain\((?P<gain>-*\d*\.*\d*)\)").expect("Failed to create regex for gain");
}

fn get_gain() -> f64 {
    let output = Command::new(MINIDSP_BINARY)
        .output()
        .expect("Failed to retrieve current gain");
    let output_string =
        String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Capture gain through regex
    let gain_capture = GAIN_REGEX
        .captures(&output_string)
        .expect("Failed to retrieve gain from regex");

    // Return the gain as a floating number
    gain_capture["gain"]
        .parse::<f64>()
        .expect("Failed to convert gain to float")
}

fn apply_gain(gain: &str) {
    println!("Setting new gain: {} dB", gain);
    Command::new(MINIDSP_BINARY)
        .arg("gain")
        .arg("--")
        .arg(gain)
        .output()
        .expect("Failed to update gain");
}

fn clamp_gain(gain: f64) -> f64 {
    clamp(gain, MINIDSP_GAIN_MIN, MINIDSP_GAIN_MAX)
}

fn different_gain(current: f64, new: f64) -> bool {
    // Stringify to miniDSP format too see if the gain has changed
    let current_gain_string = format!("{:.1}", current);
    let new_gain_string = format!("{:.1}", new);
    current_gain_string != new_gain_string
}

fn update_gain(current: f64, new: f64) -> bool {
    let gain_diff = new - current;
    let scaled_gain = current + (gain_diff * GAIN_SCALE);
    let final_gain = clamp_gain(scaled_gain);

    let gain_changed = different_gain(current, final_gain);
    if gain_changed {
        apply_gain(&format!("{:.1}", final_gain));
    }

    gain_changed
}

fn poll() {
    let mut known = false;
    let mut current_gain = MINIDSP_GAIN_MIN;

    loop {
        let new_gain = get_gain();

        if current_gain != new_gain {
            println!("Current gain: {} dB", new_gain);
        }

        // Only update the gain if we know what the last reference was
        if known {
            // Invalidate the known gain if we updated it, since it's not certain what miniDSP returns
            // (clamping/external update etc.)
            known = !update_gain(current_gain, new_gain);
        } else {
            known = true;
            current_gain = new_gain;
        }

        // Sleep until next call
        thread::sleep(time::Duration::from_millis(SLEEP_TIME_MS));
    }
}

fn change_gain(increase: bool) {
    // Retrieve current gain
    let current_gain = get_gain();
    let mut new_gain = current_gain;
    if increase {
        new_gain += GAIN_OFFSET;
    } else {
        new_gain -= GAIN_OFFSET;
    }
    // Clamp to make sure the gain is in the correct range
    let clamped_gain = clamp_gain(new_gain);
    if different_gain(current_gain, clamped_gain) {
        apply_gain(&format!("{:.1}", clamped_gain));
    }
}

fn act_on_event(event: evdev_rs::InputEvent) {
    const VOLUME_UP_KEY: evdev_rs::enums::EV_KEY = evdev_rs::enums::EV_KEY::KEY_U;
    const VOLUME_DOWN_KEY: evdev_rs::enums::EV_KEY = evdev_rs::enums::EV_KEY::KEY_D;

    // Only care about key events where the key is pressed (value = 1)
    if event.is_type(&evdev_rs::enums::EventType::EV_KEY) && event.value == 1 {
        match event.event_code {
            evdev_rs::enums::EventCode::EV_KEY(VOLUME_UP_KEY) => change_gain(true),
            evdev_rs::enums::EventCode::EV_KEY(VOLUME_DOWN_KEY) => change_gain(false),
            _ => (), // Ignore everything else
        }
    }
}

fn event(path: String) {
    let file = File::open(path).expect("Failed to open event path");
    let device = evdev_rs::Device::new_from_file(file).expect("Failed to create device");

    // Loop events
    loop {
        let (status, event) = device
            .next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING)
            .expect("Failed to read event");
        match status {
            evdev_rs::ReadStatus::Success => act_on_event(event),
            _ => panic!("Event error, aborting"),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let s = args.get(1).unwrap();
        match &s[..] {
            "event" => event(args.get(2).expect("No event path provided").to_string()),
            _ => poll(), // Poll by default
        }
    } else {
        // Poll by default
        poll();
    }
}
