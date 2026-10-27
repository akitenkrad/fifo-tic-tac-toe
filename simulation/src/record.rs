use std::any::Any;
use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use runvault::{Run, RunOptions};
use runvault_socsim::{RunHandle, RunvaultRecorder};
use serde_json::Value;
use socsim_core::Recorder;

use crate::config::Cap;

pub type RecordResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone)]
pub struct SharedRecorder(Rc<RefCell<RunvaultRecorder>>);

impl SharedRecorder {
    pub fn new(inner: Rc<RefCell<RunvaultRecorder>>) -> Self {
        Self(inner)
    }
}

impl Recorder for SharedRecorder {
    fn record_metric(&mut self, t: u64, key: &str, value: f64) {
        self.0.borrow_mut().record_metric(t, key, value);
    }

    fn record_event(&mut self, t: u64, kind: &str, payload: Value) {
        self.0.borrow_mut().record_event(t, kind, payload);
    }

    fn record_row(&mut self, t: u64, table: &str, row: &[(&str, f64)]) {
        self.0.borrow_mut().record_row(t, table, row);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

pub fn start_run(subcommand: &str, parameters: &Value, label: &str, scratch: bool) -> RecordResult<(RunvaultRecorder, RunHandle)> {
    let options = RunOptions::new("fifo-ttt", subcommand)
        .repo_id("fifo-tic-tac-toe")
        .domain("analysis")
        .scratch(scratch)
        .parameters(parameters)?
        .label(label);
    let run = Run::start(options)?;
    let recorder = RunvaultRecorder::new(run).step_unit("ply").scope("run");
    let handle = recorder.handle();
    Ok((recorder, handle))
}

pub fn metric_name(metric: &str, kx: Cap, ko: Cap) -> String {
    format!("{metric}.kx{}_ko{}", kx.label(), ko.label())
}

pub fn record_solve(handle: &RunHandle, metrics: &[(String, f64)]) -> RecordResult<()> {
    let borrowed: Vec<(&str, f64)> = metrics.iter().map(|(name, value)| (name.as_str(), *value)).collect();
    handle.with_run(|run| run.log_metrics("run", &borrowed))??;
    Ok(())
}

pub fn write_run_file(handle: &RunHandle, relative: &str, contents: &str) -> RecordResult<PathBuf> {
    let run_dir = handle.dir().ok_or("run is already closed")?;
    let path = run_dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, contents)?;
    Ok(path)
}
