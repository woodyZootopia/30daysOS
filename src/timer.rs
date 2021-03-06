use crate::asm;
use crate::fifo::{FIFO, GLOBAL_FIFO_BUF};
use alloc::{vec, vec::Vec};
use spin::Mutex;
use x86_64::instructions::port;
use x86_64::registers::rflags;

const MAX_TIMER: usize = 100;

pub struct TIMERCTL {
    /// global count of this OS. increases every 0.01s
    pub count: u32,
    /// next timing of timeout
    pub next: u32,
    pub timers: Vec<TIMER>,
    /// The index of timers used (`TimerState::Using`) now.
    /// Sorted by `next` in ascending order.
    pub used_timers: Vec<usize>,
    fifo: &'static Mutex<FIFO<u32>>,
}

use lazy_static::lazy_static;
lazy_static! {
    pub static ref TIMER_CONTROL: Mutex<TIMERCTL> = Mutex::new(TIMERCTL {
        count: 0,
        next: core::u32::MAX,
        timers: vec![TIMER::new(0); MAX_TIMER],
        used_timers: vec![],
        fifo: &GLOBAL_FIFO_BUF,
    });
}

impl TIMERCTL {
    pub fn allocate(&mut self) -> Option<usize> {
        for i in 0..MAX_TIMER {
            if self.timers[i].flag == TimerState::Unused {
                self.timers[i].flag = TimerState::Using;
                return Some(i);
            }
        }
        return None;
    }
    pub fn deallocate(&mut self, id: usize) {
        self.timers[id].flag = TimerState::Unused;
    }
    pub fn set_time(&mut self, id: usize, wait_time: u32) {
        let timeout = self.count + wait_time;
        let rf = rflags::read();
        asm::cli();
        {
            self.timers[id].timeout = timeout;
            // どこに入れればいいかを探す
            let index_to_push = self
                .used_timers
                .iter()
                .position(|&timer_idx| self.timers[timer_idx].timeout > timeout)
                .unwrap_or(self.used_timers.len());
            // 入れる
            self.used_timers.insert(index_to_push, id);

            self.next = core::cmp::min(self.next, timeout);
            self.timers[id].flag = TimerState::Using;
        }
        rflags::write(rf);
    }
    /// This function is only to be called by `interrupt.rs`. Therefore we don't need to care about
    /// interrupts
    pub fn shift_timers(&mut self) {
        if self.count >= self.next {
            let mut num_of_timeouts = 0;
            // iterate for timers in use
            for timer_id in self.used_timers.clone().iter() {
                if self.timers[*timer_id].timeout > self.count {
                    break;
                }
                // timeout happened for this timer
                num_of_timeouts += 1;
                self.push_timeout_signal(*timer_id);
                self.timers[*timer_id].flag = TimerState::Allocated;
            }
            // `num_of_timeouts` timers timed out
            self.used_timers = self.used_timers.split_off(num_of_timeouts);
            if self.used_timers.len() > 0 {
                self.next = self.timers[self.used_timers[0]].timeout;
            } else {
                self.next = core::u32::MAX;
            }
        }
    }
    pub fn push_timeout_signal(&mut self, id: usize) {
        let data = self.timers[id].data;

        let rf = rflags::read();
        asm::cli();
        self.fifo.lock().push(data).unwrap();
        rflags::write(rf);
    }
    pub fn pop(&mut self) -> Result<u32, ()> {
        let rf = rflags::read();
        asm::cli();
        let result = self.fifo.lock().pop();
        rflags::write(rf);
        result
    }
}

#[derive(Clone)]
pub struct TIMER {
    pub timeout: u32,
    pub flag: TimerState,
    pub data: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Unused,
    Allocated,
    Using,
}

impl TIMER {
    pub fn new(data: u32) -> Self {
        Self {
            timeout: 0,
            flag: TimerState::Unused,
            data,
        }
    }
}

const PIT_CTRL: u16 = 0x0043;
const PIT_CNT0: u16 = 0x0040;

pub fn init_pit() {
    let mut port_control = port::PortWriteOnly::new(PIT_CTRL);
    let mut port_counter = port::PortWriteOnly::new(PIT_CNT0);
    unsafe {
        port_control.write(0x34u8);
        port_counter.write(0x9cu8);
        port_counter.write(0x2eu8);
    }
}
