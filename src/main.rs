use std::{
    thread,
    time::{Duration, Instant},
};

use clap::Parser;
use midi_toolkit::{
    events::MIDIEvent,
    io::MIDIFile,
    pipe,
    sequence::{
        event::{cancel_tempo_events, merge_events_array, scale_event_time},
        to_vec, unwrap_items, TimeCaster,
    },
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of the midi file
    #[arg(short, long)]
    midi_file: String,
}

fn main() {
    let args = Args::parse();

    println!("evt_parsing");

    let midi = MIDIFile::open(args.midi_file, None).unwrap();

    println!("evt_parsed");

    let ppq = midi.ppq();
    let merged = pipe!(
        midi.iter_all_tracks()
        |>to_vec()
        |>merge_events_array()
        |>TimeCaster::<f64>::cast_event_delta()
        |>cancel_tempo_events(250000)
        |>scale_event_time(1.0 / ppq as f64)
        |>unwrap_items()
    );

    let now = Instant::now();
    let mut time = 0.0;

    println!("evt_playing");
    for e in merged {
        if e.delta != 0.0 {
            time += e.delta;
            let diff = time - now.elapsed().as_secs_f64();
            if diff > 0.0 {
                thread::sleep(Duration::from_secs_f64(diff));
            }
        }

        if let Some(serialized) = e.as_u32() {
            let mut chunks: Vec<String> = Vec::new();
        
            for i in (0..24).step_by(8) {
                let chunk = ((serialized >> i) & 0xFF) as u8;
                chunks.push(format!("{:02X}", chunk));
            }
        
            let joined = chunks.join(",");
            println!("{}", joined);
        }
        
    }
    println!("evt_playing_finished");
}
