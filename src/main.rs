#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use futures::channel::mpsc::{channel, Sender};
use futures_util::{future, pin_mut, StreamExt};
use mcai_worker_sdk::{
  debug, info, job::JobResult, start_worker, trace, FormatContext, Frame, JsonSchema, MessageError,
  MessageEvent, ProcessResult, StreamDescriptor, Version,
};
use stainless_ffmpeg_sys::AVMediaType;
use std::{
  convert::TryFrom,
  sync::{
    atomic::{
      AtomicUsize,
      Ordering::{Acquire, Release},
    },
    mpsc, Arc, Mutex,
  },
  thread,
  thread::JoinHandle,
  time::Duration,
};
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;

static mut FILE: File;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug, Default)]
struct TranscriptEvent {
  sequence_number: u64,
  start_time: Option<f32>,
  authot_live_id: Option<usize>,
  audio_source_sender: Option<Sender<Message>>,
  sender: Option<Arc<Mutex<mpsc::Sender<ProcessResult>>>>,
  ws_thread: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WorkerParameters {
  destination_path: String,
  source_path: String,
}

impl MessageEvent<WorkerParameters> for TranscriptEvent {
  fn get_name(&self) -> String {
    "Probe worker".to_string()
  }

  fn get_short_description(&self) -> String {
    "Worker to probe stream".to_string()
  }

  fn get_description(&self) -> String {
    r#"This worker probes stream."#.to_string()
  }

  fn get_version(&self) -> Version {
    Version::parse(built_info::PKG_VERSION).expect("unable to locate Package version")
  }

  fn init_process(
    &mut self,
    parameters: WorkerParameters,
    _format_context: Arc<Mutex<FormatContext>>,
    _sender: Arc<Mutex<mpsc::Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>, MessageError> {

    unsafe {
        FILE = File::create(parameters.destination_path)?;
    }

    Ok(vec![StreamDescriptor::new_audio(1, vec![])])
    // Ok(vec![StreamDescriptor::new_video(0, vec![]), StreamDescriptor::new_audio(1, vec![])])
  }

  fn process_frame(
    &mut self,
    job_result: JobResult,
    _stream_index: usize,
    process_frame: ProcessFrame,
  ) -> Result<ProcessResult, MessageError> {
    let transformed_subtitles = match process_frame {
      ProcessFrame::AudioVideo(frame) => {
        let size = frame.len();
        let mut pos = 0;
        while pos < size {
            unsafe {
                let bytes_written = FILE.write(&frame[pos..])?;
            }
            pos += bytes_written;
        }
      }
      _ => {
        return Err(MessageError::RuntimeError(format!(
          "Could not open frame as it was no Audio frame in job {:?}",
          job_result.get_str_job_id()
        )))
      }
    };

    Ok(())
  }


  fn ending_process(&mut self) -> Result<(), MessageError> {
    Ok(())
  }
}

fn main() {
  let worker = TranscriptEvent::default();
  start_worker(worker);
}
