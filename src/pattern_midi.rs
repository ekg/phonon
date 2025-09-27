//! MIDI and Control Pattern Operations
//!
//! Implements MIDI messages, control changes, and OSC patterns

use crate::pattern::{Fraction, Hap, Pattern, State, TimeSpan};

/// MIDI message types
#[derive(Debug, Clone)]
pub enum MidiMessage {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    PitchBend {
        channel: u8,
        value: i16,
    },
    Aftertouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    SysEx {
        data: Vec<u8>,
    },
    Clock,
    Start,
    Stop,
    Continue,
}

impl MidiMessage {
    /// Convert to raw MIDI bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            } => {
                vec![0x90 | (channel & 0x0F), *note & 0x7F, *velocity & 0x7F]
            }
            MidiMessage::NoteOff {
                channel,
                note,
                velocity,
            } => {
                vec![0x80 | (channel & 0x0F), *note & 0x7F, *velocity & 0x7F]
            }
            MidiMessage::ControlChange {
                channel,
                controller,
                value,
            } => {
                vec![0xB0 | (channel & 0x0F), *controller & 0x7F, *value & 0x7F]
            }
            MidiMessage::ProgramChange { channel, program } => {
                vec![0xC0 | (channel & 0x0F), *program & 0x7F]
            }
            MidiMessage::PitchBend { channel, value } => {
                let value = (*value + 8192).max(0).min(16383) as u16;
                vec![
                    0xE0 | (channel & 0x0F),
                    (value & 0x7F) as u8,
                    ((value >> 7) & 0x7F) as u8,
                ]
            }
            MidiMessage::Aftertouch {
                channel,
                note,
                pressure,
            } => {
                vec![0xA0 | (channel & 0x0F), *note & 0x7F, *pressure & 0x7F]
            }
            MidiMessage::ChannelPressure { channel, pressure } => {
                vec![0xD0 | (channel & 0x0F), *pressure & 0x7F]
            }
            MidiMessage::SysEx { data } => {
                let mut bytes = vec![0xF0];
                bytes.extend_from_slice(data);
                bytes.push(0xF7);
                bytes
            }
            MidiMessage::Clock => vec![0xF8],
            MidiMessage::Start => vec![0xFA],
            MidiMessage::Stop => vec![0xFC],
            MidiMessage::Continue => vec![0xFB],
        }
    }
}

/// OSC message
#[derive(Debug, Clone)]
pub struct OscMessage {
    pub address: String,
    pub args: Vec<OscValue>,
}

#[derive(Debug, Clone)]
pub enum OscValue {
    Int(i32),
    Float(f32),
    String(String),
    Blob(Vec<u8>),
    Bool(bool),
}

/// Control value with metadata
#[derive(Debug, Clone)]
pub struct ControlValue {
    pub value: f64,
    pub controller: Option<u8>,
    pub channel: Option<u8>,
}

impl Pattern<f64> {
    /// Create MIDI note pattern
    pub fn midi(self, channel: u8) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .flat_map(|hap| {
                    let note = hap.value as u8;
                    vec![
                        Hap::new(
                            hap.whole,
                            TimeSpan::new(hap.part.begin, hap.part.begin),
                            MidiMessage::NoteOn {
                                channel,
                                note,
                                velocity: 100,
                            },
                        ),
                        Hap::new(
                            hap.whole,
                            TimeSpan::new(hap.part.end, hap.part.end),
                            MidiMessage::NoteOff {
                                channel,
                                note,
                                velocity: 0,
                            },
                        ),
                    ]
                })
                .collect()
        })
    }

    /// Create control change pattern
    pub fn cc(self, controller: u8, channel: u8) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    let value = (hap.value * 127.0).max(0.0).min(127.0) as u8;
                    Hap::new(
                        hap.whole,
                        hap.part,
                        MidiMessage::ControlChange {
                            channel,
                            controller,
                            value,
                        },
                    )
                })
                .collect()
        })
    }

    /// Set control change number
    pub fn ccn(self, controller: u8) -> Pattern<ControlValue> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        ControlValue {
                            value: hap.value,
                            controller: Some(controller),
                            channel: None,
                        },
                    )
                })
                .collect()
        })
    }

    /// Set control change value
    pub fn ccv(self) -> Pattern<ControlValue> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        ControlValue {
                            value: hap.value,
                            controller: None,
                            channel: None,
                        },
                    )
                })
                .collect()
        })
    }

    /// NRPN (Non-Registered Parameter Number) control
    pub fn nrpn(self, param_msb: u8, param_lsb: u8, channel: u8) -> Pattern<Vec<MidiMessage>> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    let value = (hap.value * 16383.0).max(0.0).min(16383.0) as u16;
                    let value_msb = (value >> 7) as u8;
                    let value_lsb = (value & 0x7F) as u8;

                    Hap::new(
                        hap.whole,
                        hap.part,
                        vec![
                            MidiMessage::ControlChange {
                                channel,
                                controller: 99,
                                value: param_msb,
                            },
                            MidiMessage::ControlChange {
                                channel,
                                controller: 98,
                                value: param_lsb,
                            },
                            MidiMessage::ControlChange {
                                channel,
                                controller: 6,
                                value: value_msb,
                            },
                            MidiMessage::ControlChange {
                                channel,
                                controller: 38,
                                value: value_lsb,
                            },
                        ],
                    )
                })
                .collect()
        })
    }

    /// Set MIDI channel
    pub fn midichan(self, channel: u8) -> Pattern<(f64, u8)> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| Hap::new(hap.whole, hap.part, (hap.value, channel)))
                .collect()
        })
    }

    /// Program change
    pub fn prog_num(self, channel: u8) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    let program = (hap.value * 127.0).max(0.0).min(127.0) as u8;
                    Hap::new(
                        hap.whole,
                        hap.part,
                        MidiMessage::ProgramChange { channel, program },
                    )
                })
                .collect()
        })
    }

    /// Pitch bend pattern
    pub fn pitch_bend(self, channel: u8) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    // Convert 0-1 to -8192 to 8191
                    let value = ((hap.value * 2.0 - 1.0) * 8192.0) as i16;
                    Hap::new(
                        hap.whole,
                        hap.part,
                        MidiMessage::PitchBend { channel, value },
                    )
                })
                .collect()
        })
    }

    /// Aftertouch pattern
    pub fn aftertouch(self, note: u8, channel: u8) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    let pressure = (hap.value * 127.0).max(0.0).min(127.0) as u8;
                    Hap::new(
                        hap.whole,
                        hap.part,
                        MidiMessage::Aftertouch {
                            channel,
                            note,
                            pressure,
                        },
                    )
                })
                .collect()
        })
    }
}

impl Pattern<String> {
    /// Create OSC pattern
    pub fn osc(self, prefix: &str) -> Pattern<OscMessage> {
        let prefix = prefix.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    Hap::new(
                        hap.whole,
                        hap.part,
                        OscMessage {
                            address: format!("{}/{}", prefix, hap.value),
                            args: vec![],
                        },
                    )
                })
                .collect()
        })
    }

    /// Create OSC pattern with prefix
    pub fn osc_prefix(self, prefix: &str) -> Pattern<String> {
        let prefix = prefix.to_string();
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|mut hap| {
                    hap.value = format!("{}/{}", prefix, hap.value);
                    hap
                })
                .collect()
        })
    }
}

/// OSC pattern with arguments
pub fn osc_msg(address: &str, args: Vec<OscValue>) -> Pattern<OscMessage> {
    let address = address.to_string();
    Pattern::new(move |state: &State| {
        vec![Hap::new(
            Some(state.span),
            state.span,
            OscMessage {
                address: address.clone(),
                args: args.clone(),
            },
        )]
    })
}

/// MIDI clock pattern
pub fn midi_clock(bpm: f64) -> Pattern<MidiMessage> {
    Pattern::new(move |state: &State| {
        let ticks_per_beat = 24.0;
        let beats_per_second = bpm / 60.0;
        let ticks_per_second = ticks_per_beat * beats_per_second;

        let begin = state.span.begin.to_float();
        let end = state.span.end.to_float();
        let duration = end - begin;
        let tick_count = (duration * ticks_per_second) as usize;

        let mut haps = Vec::new();
        for i in 0..tick_count {
            let time = begin + (i as f64 / ticks_per_second);
            haps.push(Hap::new(
                Some(state.span),
                TimeSpan::new(Fraction::from_float(time), Fraction::from_float(time)),
                MidiMessage::Clock,
            ));
        }
        haps
    })
}

/// MIDI transport controls
pub fn midi_start() -> Pattern<MidiMessage> {
    Pattern::pure(MidiMessage::Start)
}

pub fn midi_stop() -> Pattern<MidiMessage> {
    Pattern::pure(MidiMessage::Stop)
}

pub fn midi_continue() -> Pattern<MidiMessage> {
    Pattern::pure(MidiMessage::Continue)
}

/// SysEx pattern
pub fn sysex(data: Vec<u8>) -> Pattern<MidiMessage> {
    Pattern::pure(MidiMessage::SysEx { data })
}

/// Create complex MIDI sequence
pub struct MidiSequence {
    pub events: Vec<(f64, MidiMessage)>,
}

impl Default for MidiSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiSequence {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn note(mut self, time: f64, note: u8, velocity: u8, duration: f64, channel: u8) -> Self {
        self.events.push((
            time,
            MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            },
        ));
        self.events.push((
            time + duration,
            MidiMessage::NoteOff {
                channel,
                note,
                velocity: 0,
            },
        ));
        self
    }

    pub fn cc(mut self, time: f64, controller: u8, value: u8, channel: u8) -> Self {
        self.events.push((
            time,
            MidiMessage::ControlChange {
                channel,
                controller,
                value,
            },
        ));
        self
    }

    pub fn to_pattern(self) -> Pattern<MidiMessage> {
        Pattern::new(move |state: &State| {
            let begin = state.span.begin.to_float();
            let end = state.span.end.to_float();

            let mut haps = Vec::new();
            for (time, msg) in &self.events {
                let event_time = begin + (time % 1.0);
                if event_time >= begin && event_time < end {
                    haps.push(Hap::new(
                        Some(state.span),
                        TimeSpan::new(
                            Fraction::from_float(event_time),
                            Fraction::from_float(event_time),
                        ),
                        msg.clone(),
                    ));
                }
            }
            haps
        })
    }
}

/// MPE (MIDI Polyphonic Expression) support
#[derive(Debug, Clone)]
pub struct MpeMessage {
    pub note: u8,
    pub velocity: u8,
    pub pitch_bend: i16,
    pub pressure: u8,
    pub timbre: u8, // CC74
    pub channel: u8,
}

impl Pattern<MpeMessage> {
    /// Convert MPE to MIDI messages
    pub fn to_midi(self) -> Pattern<Vec<MidiMessage>> {
        Pattern::new(move |state: &State| {
            let haps = self.query(state);
            haps.into_iter()
                .map(|hap| {
                    let mpe = &hap.value;
                    Hap::new(
                        hap.whole,
                        hap.part,
                        vec![
                            MidiMessage::NoteOn {
                                channel: mpe.channel,
                                note: mpe.note,
                                velocity: mpe.velocity,
                            },
                            MidiMessage::PitchBend {
                                channel: mpe.channel,
                                value: mpe.pitch_bend,
                            },
                            MidiMessage::ChannelPressure {
                                channel: mpe.channel,
                                pressure: mpe.pressure,
                            },
                            MidiMessage::ControlChange {
                                channel: mpe.channel,
                                controller: 74,
                                value: mpe.timbre,
                            },
                        ],
                    )
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_midi_pattern() {
        let p = Pattern::pure(60.0); // Middle C
        let midi = p.midi(0);

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = midi.query(&state);
        assert_eq!(haps.len(), 2); // Note on and note off

        match &haps[0].value {
            MidiMessage::NoteOn { note, .. } => assert_eq!(*note, 60),
            _ => panic!("Expected NoteOn"),
        }
    }

    #[test]
    fn test_cc_pattern() {
        let p = Pattern::pure(0.5); // 50% value
        let cc = p.cc(7, 0); // Volume on channel 0

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = cc.query(&state);
        assert_eq!(haps.len(), 1);

        match &haps[0].value {
            MidiMessage::ControlChange {
                controller, value, ..
            } => {
                assert_eq!(*controller, 7);
                assert_eq!(*value, 63); // 0.5 * 127
            }
            _ => panic!("Expected ControlChange"),
        }
    }

    #[test]
    fn test_osc_pattern() {
        let p = Pattern::from_string("trigger hit bang");
        let osc = p.osc("/drum");

        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let haps = osc.query(&state);
        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value.address, "/drum/trigger");
    }
}
