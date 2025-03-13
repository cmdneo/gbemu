use std::{
    error::Error,
    sync::mpsc::{self},
    thread,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BackendSpecificError, BuildStreamError, FromSample, Sample, SizedSample, Stream, StreamConfig,
};

use crate::log;

pub(crate) struct AudioPlayer {
    config: StreamConfig,
    sender: mpsc::Sender<Message>,
}

#[derive(Debug)]
pub(crate) struct TimedSample {
    pub(crate) timestamp: f64,
    pub(crate) left: f32,
    pub(crate) right: f32,
}

pub(crate) enum Message {
    Play,
    Pause,
    Stop,
}

impl AudioPlayer {
    pub(crate) fn new(reciever: mpsc::Receiver<TimedSample>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no audio output device found")?;
        let sup_config = device
            .default_output_config()
            .map_err(|err| err.to_string())?;
        let sample_fmt = sup_config.sample_format();
        let config = sup_config.config();

        // Stream object cannot be moved among threads(it is not Send), so
        // we create it in a dedicated thread and control it using messages.
        // It solves the issue of Emulator not being Send.
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        let (ctrl_tx, ctrl_rx) = mpsc::channel::<Message>();

        thread::spawn(move || {
            let stream = match sample_fmt {
                cpal::SampleFormat::I16 => create_stream::<i16>(&device, &config, reciever),
                cpal::SampleFormat::U16 => create_stream::<u16>(&device, &config, reciever),
                cpal::SampleFormat::F32 => create_stream::<f32>(&device, &config, reciever),
                format => Err(BuildStreamError::BackendSpecific {
                    err: BackendSpecificError {
                        description: format!("unsupported sample format: {format}"),
                    },
                }),
            };

            match stream {
                Ok(s) => {
                    tx.send(Ok(())).unwrap();
                    let status = handle_stream_control(s, ctrl_rx);

                    if let Err(err) = status {
                        log::error(&format!("audio: {}", err))
                    };
                }

                Err(err) => {
                    tx.send(Err(err.to_string())).unwrap();
                }
            }
        });

        rx.recv().unwrap()?; // propogate stream creation error, if any.

        Ok(Self {
            config: sup_config.config(),
            sender: ctrl_tx,
        })
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub(crate) fn control(&mut self, msg: Message) {
        self.sender.send(msg).unwrap();
    }
}

fn handle_stream_control(
    stream: Stream,
    reciever: mpsc::Receiver<Message>,
) -> Result<(), Box<dyn Error>> {
    stream.pause()?;

    loop {
        let Ok(msg) = reciever.recv() else {
            stream.pause()?;
            return Ok(()); // controlling thread exited.
        };

        match msg {
            Message::Play => stream.play()?,
            Message::Pause => stream.pause()?,
            Message::Stop => {
                stream.pause()?;
                return Ok(());
            }
        }
    }
}

fn create_stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: mpsc::Receiver<TimedSample>,
) -> Result<Stream, BuildStreamError> {
    let err_fn = |err| log::error(&format!("audio: stream error: {}", err));

    let channels = config.channels as usize;
    let dt = 1.0 / config.sample_rate.0 as f64;
    let mut elapsed = 0.0;

    device.build_output_stream(
        config,
        move |data: &mut [T], _| write_data(&rx, channels, dt, &mut elapsed, data),
        err_fn,
        None,
    )
}

fn write_data<T: SizedSample + FromSample<f32>>(
    rx: &mpsc::Receiver<TimedSample>,
    channels: usize,
    dt: f64,
    elapsed: &mut f64,
    frames: &mut [T],
) {
    // Fetch the latest sample and increment timer, discarding any old ones.
    // IMPORTANT: Older samples must be discarded continuously to avoid using
    // up all the memory as the channel buffers them until recieved.
    let mut fetch_n_advance = || loop {
        if let Ok(v) = rx.recv() {
            if v.timestamp >= *elapsed {
                *elapsed += dt;
                return Some(v);
            }
        } else {
            return None;
        }
    };

    match channels {
        1 => {
            for v in frames.iter_mut() {
                let Some(data) = fetch_n_advance() else {
                    return;
                };

                *v = (data.left / 2.0 + data.right / 2.0).to_sample();
            }
        }

        2 => {
            for vs in frames.chunks_mut(2) {
                let Some(data) = fetch_n_advance() else {
                    return;
                };

                vs[0] = data.left.to_sample();
                vs[1] = data.right.to_sample();
            }
        }

        _ => unimplemented!("idk how to deal with more than 2 audio channels"),
    }
}
