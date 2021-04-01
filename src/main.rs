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

fn get_gain() -> f64 {
    let output = Command::new(MINIDSP_BINARY)
        .output()
        .expect("Failed to retrieve current gain");
    let output_string =
        String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Fetch gain through regex
    let gain_regex =
        Regex::new(r"Gain\((?P<gain>-*\d*\.*\d*)\)").expect("Failed to create regex for gain");
    let capture_groups = gain_regex
        .captures(&output_string)
        .expect("Failed to retrieve gain from regex");
    let gain = &capture_groups["gain"]
        .parse::<f64>()
        .expect("Failed to convert gain to float");
    *gain
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
    let diff = new - current;
    let scaled_gain = current + (diff * GAIN_SCALE);
    let scaled_gain = clamp(scaled_gain, MINIDSP_GAIN_MIN, MINIDSP_GAIN_MAX);

    // Stringify to miniDSP format too see if the gain has changed
    let old_gain_string = format!("{:.1}", current);
    let new_gain_string = format!("{:.1}", scaled_gain);

    if old_gain_string != new_gain_string {
        // Set new gain
        apply_gain(&new_gain_string);
    }

    // True = updated gain, false = no update
    old_gain_string != new_gain_string
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
        match known {
            true => known = !update_gain(current_gain, new_gain),
            false => {
                known = true;
                current_gain = new_gain;
            }
        };

        // Sleep until next call
        thread::sleep(time::Duration::from_millis(SLEEP_TIME_MS));
    }
}
