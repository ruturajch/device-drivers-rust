#![no_std]
#![feature(allocator_api)]

extern crate kernel;

use kernel::prelude::*;
use kernel::chrdev;
use kernel::file_operations::{FileOpener, FileOperations};
use kernel::procfs::{ProcFileOperations, ProcOps};
use kernel::memory::{kmalloc, KmallocFlags};
use kernel::time::Duration;

module! {
    type: HelloWorldModule,
    name: b"hello_world",
    author: b"d0u9",
    description: b"Delay methods in Linux kernel.",
    license: b"GPL",
}

#[derive(Debug)]
struct Opt {
    show: fn(&kernel::file_operations::SeqFile, &mut Option<()>) -> isize,
    args: Option<()>,
}

struct HelloWorldModule {
    _chrdev: Option<chrdev::Registration>,
    opts: [Option<*mut Opt>; 8],
}

impl KernelModule for HelloWorldModule {
    fn init() -> Result<Self> {
        pr_warn!("HelloWorldModule loaded\n");

        let mut opts = [None, None, None, None, None, None, None, None];
        
        // Create proc entries similar to the original code
        opts[0] = Some(new_opt(jit_currentime, None));
        procfs::create("currentime", ProcFileOperations::new(jit_currentime));

        opts[1] = Some(new_opt(jit_fn, Some(JIT_BUSY)));
        procfs::create("jitbusy", ProcFileOperations::new(jit_fn));

        opts[2] = Some(new_opt(jit_fn, Some(JIT_SCHED)));
        procfs::create("jitsched", ProcFileOperations::new(jit_fn));

        opts[3] = Some(new_opt(jit_fn, Some(JIT_QUEUE)));
        procfs::create("jitqueue", ProcFileOperations::new(jit_fn));

        opts[4] = Some(new_opt(jit_fn, Some(JIT_SCHEDTO)));
        procfs::create("jitschedto", ProcFileOperations::new(jit_fn));

        opts[5] = Some(new_opt(jit_timer, None));
        procfs::create("jitimer", ProcFileOperations::new(jit_timer));

        opts[6] = Some(new_opt(jit_tasklet, None));
        procfs::create("jitasklet", ProcFileOperations::new(jit_tasklet));

        opts[7] = Some(new_opt(jit_tasklet, Some(1)));
        procfs::create("jitasklethi", ProcFileOperations::new(jit_tasklet));

        Ok(HelloWorldModule {
            _chrdev: None,
            opts,
        })
    }
}

impl Drop for HelloWorldModule {
    fn drop(&mut self) {
        pr_warn!("HelloWorldModule unloaded\n");

        // Remove proc entries
        procfs::remove("currentime");
        procfs::remove("jitbusy");
        procfs::remove("jitsched");
        procfs::remove("jitqueue");
        procfs::remove("jitschedto");
        procfs::remove("jitimer");
        procfs::remove("jitasklet");
        procfs::remove("jitasklethi");

        for opt in self.opts.iter_mut() {
            if let Some(opt_ptr) = opt.take() {
                unsafe { kmalloc::dealloc(opt_ptr as *mut u8, KmallocFlags::empty()) };
            }
        }
    }
}

fn new_opt(show: fn(&kernel::file_operations::SeqFile, &mut Option<()>) -> isize, args: Option<()>) -> *mut Opt {
    let opt = kmalloc::alloc::<Opt>(1, KmallocFlags::empty());
    unsafe {
        (*opt).show = show;
        (*opt).args = args;
    }
    opt
}
