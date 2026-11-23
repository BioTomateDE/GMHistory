use crate::ExtractGmlFunctionArgs;
use crate::error::*;
use crate::util::listdir;
use bstr::BStr;
use bstr::ByteSlice;
use std::fs::File;

const SAMPLE_FUNC: &str = "\"show_debug_message\"";
const IDENT_CHARSET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_1234567890";

pub async fn extract_gml_functions(args: ExtractGmlFunctionArgs) -> MyResult<()> {
    for entry in listdir(&args.runners_dir)? {
        if !entry.is_file() {
            continue;
        }
        let name = entry.filename();
        if !name.ends_with(".c") {
            continue;
        }

        let file = File::open(&entry.path).ctx(|| format!("Failed to open {entry}"))?;
        file.lock().ctx(|| format!("Failed to lock {entry}"))?;
        // SAFETY: File is locked. I don't care about priviliged users bypassing this advisory lock to invoke UB.
        let mmap = unsafe { ::memmap2::Mmap::map(&file) }.ctx(|| format!("Failed to mmap {entry}"))?;
        // let mmap = std::fs::read(&entry.path).unwrap();
        let gml_functions: Vec<String> =
            extract(&mmap).ctx(|| format!("Failed to extract GML functions in runner {name:?}"))?;
    }

    Ok(())
}

fn extract(c_source_code: &[u8]) -> MyResult<Vec<String>> {
    let code = BStr::new(c_source_code);

    let Some(sample_func_idx) = code.find_iter(SAMPLE_FUNC).next() else {
        // let Some(sample_func_idx) = ::memchr::memmem::find(code, SAMPLE_FUNC.as_bytes()) else {
        log::error!("Failed to find sample function {SAMPLE_FUNC} in source code");
        return Ok(vec![]);
    };

    let before = code[..sample_func_idx].trim_end();
    let before = before.strip_suffix(b"(").ctx(|| {
        format!(
            "No open paren before sample function {SAMPLE_FUNC}:\n{}",
            extract_line(sample_func_idx, code)
        )
    })?;
    let ident_start = BStr::new(before).rfind_not_byteset(IDENT_CHARSET).map_or(0, |x| x + 1);
    let ident: &str = str::from_utf8(&before[ident_start..]).unwrap();
    // Found name of GML function registration subroutine.
    // Canonical name: `Function_Add`. Otherwise `FUN_00430af0` or some shit.
    log::info!("Found name of GML function registration subroutine: {ident}");

    for idx in code.find_iter(ident) {}

    // dbg!(sample_func_idx);

    Ok(vec![])
}

fn extract_line(idx: usize, code: &'_ [u8]) -> std::borrow::Cow<'_, str> {
    let start = code[..idx].rfind(b"\n").map_or(0, |x| x + 1);
    let end = code[idx..].find(b"\n").map_or(code.len(), |x| x + idx);
    String::from_utf8_lossy(&code[start..end])
}
