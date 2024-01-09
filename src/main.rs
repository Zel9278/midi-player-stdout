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
        event::{cancel_tempo_events, merge_events_array, scale_event_time, get_channel_statistics},
        to_vec, unwrap_items, TimeCaster,
    },
};
use alsa::seq;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of the midi file
    #[arg(short, long)]
    midi_file: String,
}

const SEQ_ID:   i32 = 14;
const SEQ_PORT: i32 = 0;

fn note( s: &alsa::Seq, evt : alsa::seq::EventType,  channel : u8, note: u8, velocity : u8, s_port:i32)
{
        let o_note = alsa::seq::EvNote {
                channel,
                note,
                velocity,
                off_velocity: 0,
                duration: 0
        };
        let mut o_event = alsa::seq::Event::new( evt , &o_note );

        o_event.set_direct();
        o_event.set_source( s_port );
        o_event.set_dest( alsa::seq::Addr { client: SEQ_ID, port: SEQ_PORT } );
        let _ = s.event_output( &mut o_event ) ;
        let _ = s.drain_output();
}

fn main() {
    let args = Args::parse();

    eprintln!("evt_parsing");

    let midi = MIDIFile::open(args.midi_file, None).unwrap();

    eprintln!("evt_parsed");

    let stats = pipe!(
      midi.iter_all_tracks()  
      |>to_vec()
      |>merge_events_array()
      |>get_channel_statistics().unwrap()
    );

    eprintln!("evt_note_count,{}", stats.note_count());

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

    eprintln!("evt_playing");

    for e in merged {
        if e.delta != 0.0 {
            time += e.delta;
            let diff = time - now.elapsed().as_secs_f64();
            if diff > 0.0 {
                thread::sleep(Duration::from_secs_f64(diff));
            }
        }

        if let Some(serialized) = e.as_u32() {
            for i in (0..24).step_by(8) {
                let chunk = ((serialized >> i) & 0xFF) as u8;
                let command = chunk >> 4;
                let channel = chunk & 0x0F;

                if command == 0x09 {
                    let note_num = ((serialized >> (i + 8)) & 0xFF) as u8;
                    let velocity = ((serialized >> (i + 16)) & 0xFF) as u8;

                    if velocity == 0 {
                        note(
                            &alsa::Seq::open(None, Some(alsa::Direction::Playback), false).unwrap(),
                            seq::EventType::Noteoff,
                            channel,
                            note_num,
                            velocity,
                            0
                        );
                    }

                    note(
                        &alsa::Seq::open(None, Some(alsa::Direction::Playback), false).unwrap(),
                        seq::EventType::Noteon,
                        channel,
                        note_num,
                        velocity,
                        0
                    );
                }

                if command == 0x08 {
                    let note_num = ((serialized >> (i + 8)) & 0xFF) as u8;
                    let velocity = ((serialized >> (i + 16)) & 0xFF) as u8;

                    note(
                        &alsa::Seq::open(None, Some(alsa::Direction::Playback), false).unwrap(),
                        seq::EventType::Noteoff,
                        channel,
                        note_num,
                        velocity,
                        0
                    );
                }
            }
        }
    }
    eprintln!("evt_playing_finished");
}
