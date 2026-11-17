use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressDrawTarget;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use std::fmt::Write;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

// TODO: eta that doesnt suck ass (needs to differentiate between archive.org downloads and others)

const STYLE: &str = "{msg:36!} {bytes_per_sec:>13} {eta_mmss} [{wide_bar}] {percent:>3}%";
const STYLE_TOTAL: &str = "{msg:36!} {prefix:10!} {eta_precise} [{wide_bar}] {percent:>3}%";

/// One downloaded file progresses the total bar length by this much.
/// Files that are e.g. 45% progress the total bar length by 45/100 * SCALE.
const SCALE: u64 = 1000;

pub static BAR_MULTI: LazyLock<MultiProgress> =
    LazyLock::new(|| MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(3)));
pub static BAR_TOTAL: LazyLock<ProgressBar> =
    LazyLock::new(|| BAR_MULTI.add(ProgressBar::new(0).with_style(bar_style(STYLE_TOTAL))));

static FILES_TOTAL: AtomicUsize = AtomicUsize::new(0);
static FILES_DONE: AtomicUsize = AtomicUsize::new(0);

fn bar_style(style: &'static str) -> ProgressStyle {
    ProgressStyle::with_template(style)
        .unwrap()
        .with_key("eta_mmss", |s: &ProgressState, w: &mut dyn Write| {
            let secs = s.eta().as_secs();
            _ = if secs >= 3600 {
                write!(w, "99:99")
            } else {
                write!(w, "{:02}:{:02}", secs / 60, secs % 60)
            };
        })
        .progress_chars("=> ")
}

fn refresh_msg() {
    let done = FILES_DONE.load(Ordering::Relaxed);
    let total = FILES_TOTAL.load(Ordering::Relaxed);
    BAR_TOTAL.set_message(format!("({done}/{total})"));
}

pub fn set_download_count(count: usize) {
    FILES_TOTAL.store(count, Ordering::Relaxed);
    // FILES_DONE is already 0
    BAR_TOTAL.set_length(count as u64 * SCALE);
    BAR_TOTAL.set_position(0);
    BAR_TOTAL.set_message(format!("(0/{count})"));
}

/// How much of the total bar this file has earned so far.
#[allow(clippy::manual_checked_ops)]
const fn share(pos: u64, len: u64) -> u64 {
    if len == 0 { 0 } else { (pos * SCALE) / len }
}

pub struct DownloadBar(ProgressBar);

impl DownloadBar {
    pub fn new(name: String) -> Self {
        let bar = ProgressBar::new(0).with_style(bar_style(STYLE)).with_message(name);
        Self(BAR_MULTI.insert_before(&BAR_TOTAL, bar))
    }

    pub fn set_length(&self, len: u64) {
        self.0.set_length(len);
    }

    pub fn inc(&self, n: u64) {
        let len = self.0.length().unwrap_or(0);
        let before = share(self.0.position(), len);
        self.0.inc(n);
        // Cumulative difference, so integer rounding never drifts; a finished file adds exactly SCALE.
        BAR_TOTAL.inc(share(self.0.position(), len) - before);
    }

    pub fn set_completed(&self) {
        self.0.set_length(1);
        self.0.set_position(1);
    }
}

impl Drop for DownloadBar {
    fn drop(&mut self) {
        let pos = self.0.position();
        let len = self.0.length().unwrap_or(0);
        self.0.finish_and_clear();

        if len > 0 && pos >= len {
            let _: usize = FILES_DONE.fetch_add(1, Ordering::Relaxed);
        } else {
            // Failed or cancelled: take its partial share back out (e.g. before a retry).
            BAR_TOTAL.set_position(BAR_TOTAL.position().saturating_sub(share(pos, len)));
        }
        refresh_msg();
    }
}
