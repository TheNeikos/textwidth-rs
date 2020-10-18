use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr;
use x11::xlib;

/// XError holds the X11 error message
#[derive(Debug)]
struct XError(String);

impl fmt::Display for XError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "X Error: {}", self.0)
	}
}

impl Error for XError {}

enum Data {
	FontSet {
		display: *mut xlib::Display,
		fontset: xlib::XFontSet,
	},
	XFont {
		display: *mut xlib::Display,
		xfont: *mut xlib::XFontStruct,
	},
}

/// A context, holding the internal data required to query a string
pub struct Context {
	data: Data,
}

impl Context {
	/// Creates a new context given by the font string given here.
	///
	/// The font string should be of the X11 form, as selected by `fontsel`.
	/// XFT is not supported!
	pub fn new(name: &str) -> Result<Self, Box<dyn Error>> {
		unsafe {
			let name: CString = CString::new(name)?;
			let dpy = xlib::XOpenDisplay(ptr::null());
			if dpy.is_null() {
				return Err(Box::new(XError("Could not open display".into())));
			}
			let mut missing_ptr = MaybeUninit::uninit();
			let mut missing_len = MaybeUninit::uninit();
			let fontset = xlib::XCreateFontSet(
				dpy,
				name.as_ptr(),
				missing_ptr.as_mut_ptr(),
				missing_len.as_mut_ptr(),
				ptr::null_mut(),
			);
			if !missing_ptr.assume_init().is_null() {
				xlib::XFreeStringList(missing_ptr.assume_init());
			}
			if !fontset.is_null() {
				Ok(Context {
					data: Data::FontSet {
						display: dpy,
						fontset,
					},
				})
			} else {
				let xfont = xlib::XLoadQueryFont(dpy, name.as_ptr());
				if xfont.is_null() {
					xlib::XCloseDisplay(dpy);
					Err(Box::new(XError(format!(
						"Could not load font: {:?}",
						name
					))))
				} else {
					Ok(Context {
						data: Data::XFont {
							display: dpy,
							xfont,
						},
					})
				}
			}
		}
	}

	/// Creates a new context with the misc-fixed font.
	pub fn with_misc() -> Result<Self, Box<dyn Error>> {
		Self::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*")
	}

	/// Get text width for the given string
	pub fn text_width<S: AsRef<str>>(&self, text: S) -> u64 {
		get_text_width(&self, text)
	}
}

impl Drop for Context {
	fn drop(&mut self) {
		unsafe {
			match self.data {
				Data::FontSet { display, fontset } => {
					xlib::XFreeFontSet(display, fontset);
					xlib::XCloseDisplay(display);
				}
				Data::XFont { display, xfont } => {
					xlib::XFreeFont(display, xfont);
					xlib::XCloseDisplay(display);
				}
			}
		}
	}
}

/// Get the width of the text rendered with the font specified by the context
pub fn get_text_width<S: AsRef<str>>(ctx: &Context, text: S) -> u64 {
	let text = CString::new(text.as_ref()).expect("Could not create CString");
	unsafe {
		match ctx.data {
			Data::FontSet { fontset, .. } => {
				let mut rectangle = MaybeUninit::uninit();
				xlib::XmbTextExtents(
					fontset,
					text.as_ptr(),
					text.as_bytes().len() as i32,
					ptr::null_mut(),
					rectangle.as_mut_ptr(),
				);
				rectangle.assume_init().width as u64
			}
			Data::XFont { xfont, .. } => xlib::XTextWidth(
				xfont,
				text.as_ptr(),
				text.as_bytes().len() as i32,
			) as u64,
		}
	}
}

/// Sets up xlib to be multithreaded
///
/// Make sure you call this before doing __anything__ else xlib related.
/// Also, do not call this more than once preferably
pub fn setup_multithreading() {
	unsafe {
		xlib::XInitThreads();
	}
}

#[cfg(test)]
mod test {
	use super::{get_text_width, Context};
	use std::sync::Once;
	use x11::xlib;
	static SETUP: Once = Once::new();
	// THIS MUST BE CALLED AT THE BEGINNING OF EACH TEST TO MAKE SURE THAT IT IS THREAD-SAFE!!!
	fn setup() {
		SETUP.call_once(|| unsafe {
			xlib::XInitThreads();
		})
	}
	#[test]
	fn test_context_new() {
		setup();
		let ctx = Context::with_misc();
		assert!(ctx.is_ok());
	}
	#[test]
	fn test_context_drop() {
		setup();
		let ctx = Context::with_misc();
		drop(ctx);
		assert!(true);
	}
	#[test]
	fn test_text_width() {
		setup();
		let ctx = Context::with_misc().unwrap();
		assert!(get_text_width(&ctx, "Hello World") > 0);
	}
	#[test]
	fn test_text_alternate() {
		setup();
		let ctx = Context::new("?");
		assert!(ctx.is_err());
	}
}
