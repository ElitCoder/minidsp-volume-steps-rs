use num::clamp;
use regex::Regex;
use std::{process::Command, thread, time};

const MINIDSP_BINARY: &str = "/home/niklas/Documents/Programming/minidsp-rs/target/release/minidsp";
const GAIN_SCALE: f64 = 3.0;
const SLEEP_TIME: u64 = 250;

fn get_current_gain() -> f64 {
    let output = Command::new(MINIDSP_BINARY)
        .output()
        .expect("Failed to retrieve current gain");
    let output_string = String::from_utf8(output.stdout).expect("Failed to convert to string");

    // Fetch gain through regex
    let gain_regex =
        Regex::new(r"Gain\((?P<gain>-*\d*\.*\d*)\)").expect("Failed to create gain regex");
    let captured_gain = gain_regex
        .captures(&output_string)
        .expect("Failed to retrieve gain from regex");
    //println!("{}", &captured_gain["gain"]);
    let float_gain = &captured_gain["gain"]
        .parse::<f64>()
        .expect("Failed to convert gain to float");
    *float_gain
}

fn apply_new_gain(gain: f64) {
    println!("Setting new gain: {}", gain);
    Command::new(MINIDSP_BINARY)
        .arg("gain")
        .arg("--")
        .arg(format!("{:.1}", gain))
        .output()
        .expect("Failed to update new gain");
}

fn set_new_gain(gain_is_known: &mut bool, gain_current: &mut f64, gain_new: f64) {
    // Difference in gain
    let diff = gain_new - *gain_current;

    let update_gain = clamp(*gain_current + (diff * (GAIN_SCALE - 1.0)), -127.0, 0.0);

    if format!("{:.1}", gain_current) != format!("{:.1}", update_gain) {
        apply_new_gain(update_gain);
        *gain_is_known = false;
    }
}

fn main() {
    let mut gain_is_known = false;
    let mut gain_current = 0.0;

    loop {
        let new_gain = get_current_gain();
        println!("Current gain: {}", new_gain);

        match gain_is_known {
            true => set_new_gain(&mut gain_is_known, &mut gain_current, new_gain),
            false => {
                gain_is_known = true;
                gain_current = new_gain;
            }
        };

        println!("");
        // Sleep until next call
        thread::sleep(time::Duration::from_millis(SLEEP_TIME));
    }
}
