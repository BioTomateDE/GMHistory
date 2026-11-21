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

pub fn init_total_bar(count: usize, length: u64) {
    FILES_TOTAL.store(count, Ordering::Relaxed);
    // FILES_DONE is already 0
    BAR_TOTAL.set_length(length);
    BAR_TOTAL.set_position(0);
    BAR_TOTAL.set_message(format!("(0/{count})"));
}

pub fn inc_and_update_message() {
    let done: usize = FILES_DONE.fetch_add(1, Ordering::Relaxed) + 1;
    let total = FILES_TOTAL.load(Ordering::Relaxed);
    BAR_TOTAL.set_message(format!("({done}/{total})"));
}

pub struct DownloadBar(ProgressBar);

impl DownloadBar {
    pub fn new(name: String, length: u64) -> Self {
        let bar = ProgressBar::new(length).with_style(bar_style(STYLE)).with_message(name);
        Self(BAR_MULTI.insert_before(&BAR_TOTAL, bar))
    }

    pub fn inc(&self, n: u64) {
        self.0.inc(n);
        BAR_TOTAL.inc(n);
    }
}

impl Drop for DownloadBar {
    fn drop(&mut self) {
        self.0.finish_and_clear();
        inc_and_update_message();
    }
}
