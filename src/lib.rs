extern crate x11;
#[macro_use] extern crate failure;

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::slice;
use std::ptr;
use std::mem;

use x11::xlib;

pub enum Context {
    FontSet {
        display: *mut xlib::Display,
        fontset: xlib::XFontSet,
    },
    XFont {
        display: *mut xlib::Display,
        xfont: *mut xlib::XFontStruct,
    }
}

impl Context {
    fn new(name: &str) -> Result<Context, failure::Error> {
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
                return Ok(Context::FontSet {
                    display: dpy,
                    fontset: fontset,
                });
            } else {
                let xfont = xlib::XLoadQueryFont(dpy, name.as_ptr());

                if xfont.is_null() {
                    return Err(format_err!("Could not load font: {:?}", name))?;
                }

                return Ok(Context::XFont {
                    display: dpy,
                    xfont: xfont,
                });
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            match *self {
                Context::FontSet { display, fontset } => {
                    xlib::XFreeFontSet(display, fontset);
                    xlib::XCloseDisplay(display);
                }
                Context::XFont { display, xfont } => {
                    xlib::XFreeFont(display, xfont);
                    xlib::XCloseDisplay(display);
                }
            }
        }
    }
}

pub fn get_text_width<S: AsRef<str>>(ctx: &Context, text: S) -> u64 {
    let text = CString::new(text.as_ref()).expect("Could not create cstring");

    unsafe {

        match *ctx {
            Context::FontSet { display, fontset } => {
                let mut r = mem::uninitialized();
                xlib::XmbTextExtents(fontset, text.as_ptr(),
                                     text.as_bytes().len() as i32, ptr::null_mut(), &mut r);
                return r.width as u64;
            }
            Context::XFont { xfont, .. } => {
                return xlib::XTextWidth(xfont, text.as_ptr(), text.as_bytes().len() as i32) as u64;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Context, get_text_width};

    #[test]
    fn test_context_new() {
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*");
        assert!(ctx.is_ok());
    }

    #[test]
    fn test_context_drop() {
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*");
        drop(ctx);
        assert!(true);
    }

    #[test]
    fn test_text_width() {
        let ctx = Context::new("-misc-fixed-*-*-*-*-*-*-*-*-*-*-*-*").unwrap();

        assert!(get_text_width(&ctx, "Hello World") > 0);
    }

}
