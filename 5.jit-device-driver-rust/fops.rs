#![no_std]
#![feature(allocator_api)]

extern crate kernel;

use kernel::prelude::*;
use kernel::procfs::{ProcFileOperations, ProcOps};
use kernel::memory::{kmalloc, KmallocFlags};
use kernel::time::{Duration, jiffies, get_jiffies_64};
use kernel::sched::{schedule, TaskState};
use kernel::file_operations::{FileOpener, FileOperations};
use kernel::waitqueue::WaitQueue;
use kernel::tasklet::Tasklet;
use kernel::timer::{Timer, TimerFn};

module! {
    type: DelayMethodsModule,
    name: b"delay_methods",
    author: b"d0u9",
    description: b"Delay methods in Linux kernel",
    license: b"GPL",
}

#[derive(Debug)]
struct Opt {
    show: fn(&kernel::file_operations::SeqFile, &mut Option<()>) -> isize,
    args: Option<()>,
}

struct DelayMethodsModule {
    opts: [Option<*mut Opt>; 8],
}

impl KernelModule for DelayMethodsModule {
    fn init() -> Result<Self> {
        pr_warn!("DelayMethodsModule loaded\n");

        let mut opts = [None, None, None, None, None, None, None, None];

        // Create proc entries
        opts[0] = Some(new_opt(jit_currentime, None));
        opts[1] = Some(new_opt(jit_fn, Some(JIT_BUSY)));
        opts[2] = Some(new_opt(jit_fn, Some(JIT_SCHED)));
        opts[3] = Some(new_opt(jit_fn, Some(JIT_QUEUE)));
        opts[4] = Some(new_opt(jit_fn, Some(JIT_SCHEDTO)));
        opts[5] = Some(new_opt(jit_timer, None));
        opts[6] = Some(new_opt(jit_tasklet, None));
        opts[7] = Some(new_opt(jit_tasklet, Some(1)));


        // Register proc files
        for (i, opt) in opts.iter_mut().enumerate() {
            if let Some(opt) = opt {
                proc_create_data!("currentime", &proc_ops, opt);
            }
        }

        Ok(DelayMethodsModule { opts })
    }
}

impl Drop for DelayMethodsModule {
    fn drop(&mut self) {
        pr_warn!("DelayMethodsModule unloaded\n");

        for i in 0..self.opts.len() {
            if let Some(opt) = &self.opts[i] {
                kfree(opt as *mut _);
            }
        }
    }
}

fn jit_currentime(m: &kernel::file_operations::SeqFile, _: &mut Option<()>) -> isize {
    let j1 = jiffies();
    let j2 = get_jiffies_64();
    let tv1 = kernel::time::ktime_get_real_ts64();
    let tv2 = kernel::time::ktime_get_coarse_real_ts64();

    pr_debug!("{}() is invoked", __FUNCTION__);

    seq_printf!(m, 
        "0x{:08x} 0x{:016x} {:10} {:06}\n{:41} {:09}\n", 
        j1, j2,
        tv1.tv_sec as i32, tv1.tv_nsec as i32,
        tv2.tv_sec as i32, tv2.tv_nsec as i32
    );

    0
}

fn jit_fn(m: &kernel::file_operations::SeqFile, p: &mut Option<()>) -> isize {
    let mut wait = WaitQueue::new();
    let j0 = jiffies();
    let j1 = j0 + delay;  // `delay` should be defined elsewhere, just like in the C code

    pr_debug!("{}() is invoked", __FUNCTION__);

    match *p {
        Some(JIT_BUSY) => {
            while time_before(jiffies(), j1) {
                cpu_relax();
            }
        },
        Some(JIT_SCHED) => {
            while time_before(jiffies(), j1) {
                schedule();
            }
        },
        Some(JIT_QUEUE) => {
            wait_event_interruptible_timeout(wait, 0, delay);
        },
        Some(JIT_SCHEDTO) => {
            set_current_state(TaskState::Interruptible);
            schedule_timeout(delay);
        },
        _ => pr_debug!("Known option"),
    }

    seq_printf!(m, "{:9} {:9}\n", j0, j1);

    0
}

#[derive(Debug)]
struct JitData {
    timer: Timer,
    tlet: Tasklet,
    wait: WaitQueue,
    prevjiffies: u64,
    buf: Option<Vec<u8>>,
    loops: i32,
}

fn jit_timer_fn(t: &Timer) {
    let data = t.data();
    let j = jiffies();

    pr_debug!("{}() is invoked", __FUNCTION__);

    data.buf.push_str(&format!(
        "{:9}  {:3}     {}    {}   {}   {}\n",
        j, j - data.prevjiffies,
        in_interrupt() as i32,
        current_pid(),
        smp_processor_id(),
        current_comm()
    ));

    if data.loops > 0 {
        data.timer.expires += tdelay;
        data.prevjiffies = j;
        add_timer(&data.timer);
    } else {
        wake_up_interruptible(&data.wait);
    }
}

fn jit_timer(m: &kernel::file_operations::SeqFile, _: &mut Option<()>) -> isize {
    let mut data = JitData {
        timer: Timer::new(),
        tlet: Tasklet::new(),
        wait: WaitQueue::new(),
        prevjiffies: jiffies(),
        buf: Some(vec![0; PAGE_SIZE]),
        loops: JIT_ASYNC_LOOPS,
    };

    let mut buf = vec![0; PAGE_SIZE];
    let mut buf2 = &mut buf[..];

    timer_setup(&data.timer, jit_timer_fn, 0);
    init_waitqueue_head(&data.wait);

    buf2.write_fmt(format_args!(
        "   time   delta  inirq    pid   cpu command\n"
    ))?;

    buf2.write_fmt(format_args!(
        "{:9}  {:3}     {}    {}   {}   {}\n",
        jiffies(), 0,
        in_interrupt() as i32,
        current_pid(),
        smp_processor_id(),
        current_comm()
    ))?;

    data.buf = Some(buf2);

    data.timer.expires = jiffies() + tdelay;
    add_timer(&data.timer);

    wait_event_interruptible(data.wait, data.loops == 0);

    seq_printf!(m, "{}", buf);

    0
}
