use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr;
use thiserror::Error;
use x11::xlib;

/// XError holds the X11 error message
#[derive(Debug, Error)]
pub enum XError {
    /// No X11 display found
    #[error("X Error: Could not open Display")]
    DisplayOpen,

    /// The font could not be found
    #[error("X Error: Could not load font with name {0:?}")]
    CouldNotLoadFont(CString),

    /// This error is returned when the string you pass cannot be converted to a CString
    #[error("CStrings cannot hold NUL values")]
    NulError(#[from] std::ffi::NulError),
}

static_assertions::assert_impl_all!(XError: Sync, Send);

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
    pub fn new(name: &str) -> Result<Self, XError> {
        let name: CString = CString::new(name)?;
        // SAFE because we simply call the
        let dpy = unsafe { xlib::XOpenDisplay(ptr::null()) };
        if dpy.is_null() {
            return Err(XError::DisplayOpen);
        }
        let mut missing_ptr = MaybeUninit::uninit();
        let mut missing_len = MaybeUninit::uninit();
        // SAFE because values are correct
        let fontset = unsafe {
            xlib::XCreateFontSet(
                dpy,
                name.as_ptr(),
                missing_ptr.as_mut_ptr(),
                missing_len.as_mut_ptr(),
                ptr::null_mut(),
            )
        };

        // SAFE because XCreateFontSet always sets both ptrs to NULL or a valid value
        unsafe {
            if !missing_ptr.assume_init().is_null() {
                xlib::XFreeStringList(missing_ptr.assume_init());
            }
        }
        if !fontset.is_null() {
            Ok(Context {
                data: Data::FontSet {
                    display: dpy,
                    fontset,
                },
            })
        } else {
            // SAFE as both dpy and name are valid
            let xfont = unsafe { xlib::XLoadQueryFont(dpy, name.as_ptr()) };
            if xfont.is_null() {
                // SAFE as dpy is a valid display
                unsafe { xlib::XCloseDisplay(dpy) };
                Err(XError::CouldNotLoadFont(name))
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

    /// Creates a new context with the misc-fixed font.
    pub fn with_misc() -> Result<Self, XError> {
        Self::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*")
    }

    /// Get text width for the given string
    pub fn text_width<S: AsRef<str>>(&self, text: S) -> Result<u64, XError> {
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
pub fn get_text_width<S: AsRef<str>>(ctx: &Context, text: S) -> Result<u64, XError> {
    let text = CString::new(text.as_ref())?;
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
                Ok(rectangle.assume_init().width as u64)
            }
            Data::XFont { xfont, .. } => {
                Ok(xlib::XTextWidth(xfont, text.as_ptr(), text.as_bytes().len() as i32) as u64)
            }
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
        assert!(get_text_width(&ctx, "Hello World").unwrap() > 0);
    }
    #[test]
    fn test_text_alternate() {
        setup();
        let ctx = Context::new("?");
        assert!(ctx.is_err());
    }
}
