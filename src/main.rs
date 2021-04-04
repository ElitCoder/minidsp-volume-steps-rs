use lazy_static::lazy_static;
use num::clamp;
use regex::Regex;
use std::{process::Command, thread, time};

// Path to control software
const MINIDSP_BINARY: &str = "minidsp";

// Factor to scale gain differences with
const GAIN_SCALE: f64 = 6.0;

// Time to sleep between calls
const SLEEP_TIME_MS: u64 = 50;

// Gain values for miniDSP 2x4 HD
const MINIDSP_GAIN_MIN: f64 = -127.0;
const MINIDSP_GAIN_MAX: f64 = 0.0;

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

fn update_gain(current: f64, new: f64) -> bool {
    let gain_diff = new - current;
    let scaled_gain = current + (gain_diff * GAIN_SCALE);
    let final_gain = clamp(scaled_gain, MINIDSP_GAIN_MIN, MINIDSP_GAIN_MAX);

    // Stringify to miniDSP format too see if the gain has changed
    let old_gain_string = format!("{:.1}", current);
    let new_gain_string = format!("{:.1}", final_gain);
    let gain_changed = old_gain_string != new_gain_string;

    if gain_changed {
        apply_gain(&new_gain_string);
    }

    // Return true if the gain was changed, false otherwise
    gain_changed
}

fn main() {
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
