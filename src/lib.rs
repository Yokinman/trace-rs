use std::backtrace;
use std::sync::Mutex;

/// Prints to the standard output, indented by the size of the local call stack.
/// 
/// Equivalent to the [`println!`] macro, except that the message is indented by
/// its relation to the previous `trace!` call, and the first frame in the call
/// stack that exists in the same crate as a past `trace!` caller.
/// 
/// Note that this macro utilizes [`std::backtrace`] which may be performance
/// intensive and inconsistent, especially across platforms. Currently, `trace!`
/// also blocks threads, as the Debug implementation of Backtrace is blocking.
/// 
/// This macro is fully equivalent to [`println!`] if the `RUST_BACKTRACE` or
/// `RUST_LIB_BACKTRACE` environment variables are both not set (or if the
/// call stack otherwise couldn't be captured), avoiding the performance cost.
/// 
/// [`println!`]: std::println
/// 
/// # Panics
///
/// Panics if writing to [`std::io::stdout`] fails.
///
/// Writing to non-blocking stdout can cause an error, which will lead
/// this macro to panic.
/// 
/// # Indentation Symbols
/// 
/// Each indentation of four characters represents how the current thread's call
/// stack compares to the one from the previous `trace!`.
/// 
/// - `    ` indicates that the call stack matches up to this depth.
/// - `>---` indicates that the call stack differs at or before this depth.
/// - `@   ` or `@---` marks the baseline depth, like a main function or thread.
/// - `|   ` marks the current depth relative to the baseline.
/// 
/// # Examples
/// 
/// ```
/// use trace::trace;
/// std::env::set_var("RUST_LIB_BACKTRACE", "1");
/// 
/// fn s(n: u8, k: u8) -> u8 {
///     trace!("n:{n}, k:{k}");
///     if n == k {
///         return 1
///     }
///     if k == 0 || n < k {
///         return 0
///     }
///     s(n-1, k-1) + s(n-1, k)*k
/// }
/// 
/// trace!("# of ways to group 3 items into 2 unordered sets:");
/// trace!("Result: {}", s(3, 2));
/// ```
/// Output:
/// ```text
/// @---|   # of ways to group 3 items into 2 unordered sets:
///     >---|   n:3, k:2
///         >---|   n:2, k:1
///             >---|   n:1, k:0
///                 |   n:1, k:1
///             |   n:2, k:2
///     |   Result: 3
/// ```
#[macro_export]
macro_rules! trace {
	() => {
		$crate::trace!("")
	};
	($($arg:tt)*) => {{
		$crate::_trace(format!($($arg)*), module_path!());
	}};
}

#[doc(hidden)]
pub fn _trace(text: String, module_path: &str) {
	//! Utility function for the [`trace!`] macro.
	//! 
	//! [`trace!`]: crate::trace::trace
	
	let trace_capture = backtrace::Backtrace::capture();
	if trace_capture.status() != backtrace::BacktraceStatus::Captured {
		println!("{text}");
		return
	}
	
	static LAST_TRACE_INFO: Mutex<(Vec<String>, usize)> = Mutex::new((vec![], 0));
	let (last_trace, basis_depth) = &mut *LAST_TRACE_INFO.lock().unwrap();
	let last_trace_depth = last_trace.len();
	
	let trace_string = format!("{:?}", trace_capture);
	let trace        = trace_string.rsplit('}');
	let trace_path   = &format!("fn: \"{}::_trace\"", module_path!());
	let trace_size   = trace.size_hint();
	last_trace.reserve(trace_size.1.unwrap_or(trace_size.0).saturating_sub(last_trace.capacity()));
	
	let crate_name = module_path.split("::").next().unwrap();
	let crate_path = &format!("fn: \"{}::", crate_name);
	
	let mut trace_depth = 0;
	let mut match_depth = 0;
	let mut crate_depth = 0;
	
	'find_depth: for frame in trace {
		if frame.contains(trace_path) {
			break 'find_depth
		}
		if crate_depth == 0 && frame.contains(crate_path) {
			crate_depth = trace_depth;
		}
		if trace_depth < last_trace_depth {
			if match_depth == trace_depth && frame == last_trace[trace_depth] {
				match_depth += 1;
			}
			last_trace[trace_depth] = frame.to_owned();
		} else {
			last_trace.push(frame.to_owned());
		}
		trace_depth += 1;
	}
	if trace_depth == 0 {
		println!("{text}");
		return
	}
	last_trace.truncate(trace_depth);
	trace_depth -= 1;
	match_depth = match_depth.min(trace_depth);
	
	 // Print Line w/ Indentation:
	let mut depth_text = String::new();
	if match_depth == 0 || match_depth < *basis_depth {
		*basis_depth = if crate_depth == 0 {
			trace_depth
		} else {
			crate_depth
		};
		if trace_depth > *basis_depth {
			depth_text += "@---";
			depth_text += &">---".repeat(trace_depth - *basis_depth - 1);
			depth_text += "|   ";
		} else {
			depth_text += "@   ";
		}
	} else {
		depth_text += &"    ".repeat(match_depth - *basis_depth);
		depth_text += &">---".repeat(trace_depth - match_depth);
		depth_text += "|   ";
	}
	depth_text += & if text.contains('\n') {
		text.replace('\n', &format!("\n{}|   ", "    ".repeat(trace_depth - *basis_depth)))
	} else {
		text
	};
	println!("{depth_text}");
}