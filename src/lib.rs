extern crate x11;
#[macro_use] extern crate failure;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::mem;

use x11::xlib;

enum Data {
    FontSet {
        display: *mut xlib::Display,
        fontset: xlib::XFontSet,
    },
    XFont {
        display: *mut xlib::Display,
        xfont: *mut xlib::XFontStruct,
    }
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
    pub fn new(name: &str) -> Result<Context, failure::Error> {
        unsafe {
            let name : CString = CString::new(name)?;

            let dpy = xlib::XOpenDisplay(ptr::null());
            if dpy.is_null() {
                return Err(format_err!("Could not open display"));
            }

            let missing_ptr: *mut *mut c_char = mem::uninitialized();
            let missing_len: *mut c_int = mem::uninitialized();
            let fontset = xlib::XCreateFontSet(dpy, name.as_ptr(),
                                        mem::transmute(&missing_ptr), mem::transmute(&missing_len),
                                        ptr::null_mut());

            if !missing_ptr.is_null() {
                xlib::XFreeStringList(missing_ptr);
            }

            if !fontset.is_null() {
                return Ok(Context {
                    data: Data::FontSet {
                        display: dpy,
                        fontset: fontset,
                    }
                });
            } else {
                let xfont = xlib::XLoadQueryFont(dpy, name.as_ptr());

                if xfont.is_null() {
                    xlib::XCloseDisplay(dpy);
                    return Err(format_err!("Could not load font: {:?}", name))?;
                }

                return Ok(Context {
                    data: Data::XFont {
                        display: dpy,
                        xfont: xfont,
                    }
                });
            }
        }
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
    let text = CString::new(text.as_ref()).expect("Could not create cstring");

    unsafe {

        match ctx.data {
            Data::FontSet { fontset, .. } => {
                let mut r = mem::uninitialized();
                xlib::XmbTextExtents(fontset, text.as_ptr(),
                                     text.as_bytes().len() as i32, ptr::null_mut(), &mut r);
                return r.width as u64;
            }
            Data::XFont { xfont, .. } => {
                return xlib::XTextWidth(xfont, text.as_ptr(), text.as_bytes().len() as i32) as u64;
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
    use super::{Context, get_text_width};
    use std::sync::{Once, ONCE_INIT};
    use x11::xlib;

    static SETUP: Once = ONCE_INIT;

    // THIS MUST BE CALLED AT THE BEGINNING OF EACH TEST TO MAKE SURE THAT IT IS THREAD-SAFE!!!
    fn setup() {
        SETUP.call_once(|| {
            unsafe {
                xlib::XInitThreads();
            }
        })
    }


    #[test]
    fn test_context_new() {
        setup();
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*");
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_context_drop() {
        setup();
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*");
        drop(ctx);
        assert!(true);
    }

    #[test]
    fn test_text_width() {
        setup();
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*").unwrap();

        assert!(get_text_width(&ctx, "Hello World") > 0);
    }

    #[test]
    fn test_text_alternate() {
        setup();
        let ctx = Context::new("basdkladslk");
        assert!(ctx.is_err());
    }
}
