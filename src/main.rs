use std::fmt::Display;
use std::fs::File;
use std::io::{IoSliceMut, BufReader, BufRead, IoSlice};
use nix::sys::uio::{process_vm_readv, process_vm_writev, RemoteIoVec};
use nix::unistd::getpid;
use nix::unistd::Pid;


struct MemoryReader{
    pid: Pid,
    heap_address_start: usize, 
    heap_address_end: usize, 
    stack_address_start: usize, 
    stack_address_end: usize, 
    heap_data: Vec<(usize, [u8; 4])>,
    stack_data: Vec<(usize, [u8; 4])>,
}


impl MemoryReader{
    fn new<T: Into<Pid> + Display + Copy>(provided_pid: T) -> Self 
    {
        let mut m = MemoryReader {
            pid: provided_pid.into(), 
            heap_address_start: 0, 
            heap_address_end: 0, 
            stack_address_start: 0, 
            stack_address_end: 0,
            heap_data: Vec::new(),
            stack_data: Vec::new()
        };

        let file = File::open(format!("/proc/{}/maps", provided_pid)).expect(format!("No process with {} found.", provided_pid).as_str());
        let reader = BufReader::new(file);

        for result in reader.lines() {
            if let Ok(line) = result {
                if line.trim().ends_with("[stack]") {
                    let split_line: Vec<&str> = line.split("-").collect();
                    let split_line_end: Vec<&str> = split_line[1].split(" ").collect();

                    m.stack_address_start = usize::from_str_radix(split_line[0], 16).unwrap();
                    m.stack_address_end = usize::from_str_radix(split_line_end[0], 16).unwrap();
                } else if line.trim().ends_with("[heap]") {
                    let split_line: Vec<&str> = line.split("-").collect();
                    let split_line_end: Vec<&str> = split_line[1].split(" ").collect();

                    m.heap_address_start = usize::from_str_radix(split_line[0], 16).unwrap();
                    m.heap_address_end = usize::from_str_radix(split_line_end[0], 16).unwrap();
                }
            } else {
                println!("No Stack and Heap address to Read!")
            }
        };
        m
    }

    fn read_heap(&mut self) -> &Vec<(usize, [u8; 4])> {
        let mut data = [0; 4];
        let mut remote_vec = RemoteIoVec {
            base: self.heap_address_start,
            len: 4
        };
        while remote_vec.base < self.heap_address_end{
            let buf = IoSliceMut::new(&mut data);
            if let Ok(_) = process_vm_readv(self.pid, &mut[buf], &[remote_vec]) {
                self.heap_data.push((remote_vec.base, data));
            }
            remote_vec.base += 4;
        };
        return &self.heap_data
    }

    fn read_stack(&mut self) -> &Vec<(usize, [u8; 4])> {
        let mut data = [0; 4];
        let mut remote_vec = RemoteIoVec {
            base: self.stack_address_start,
            len: 4
        };
        while remote_vec.base < self.stack_address_end{
            let buf = IoSliceMut::new(&mut data);
            if let Ok(_) = process_vm_readv(self.pid, &mut[buf], &[remote_vec]) {
                self.stack_data.push((remote_vec.base, data));
            }
            remote_vec.base += 4;
        };
        return &self.stack_data
    }
}


fn main() {
    let pid = getpid();

    let string_ = String::from("abcd");
    let number: u32 = 1000_000_000;
    println!("Values before manipulation of memory: (heap: {}) (stack: {})", string_, number);

    let mut m = MemoryReader::new(pid);
    let mut vec_ =m.read_heap();

    for (address, data) in m.read_heap() {
        if data == string_.as_bytes() {
            let value = [data[0] + 1, data[1] + 2, data[2] + 3, data[3] + 4];
            let buf = IoSlice::new(&value);
            let r_iov = RemoteIoVec {
                base: *address,
                len: 4
            };
            process_vm_writev(pid,  &[buf], &[r_iov]).unwrap();
        }
    }

    for (address, data) in m.read_stack() {
        let num = u32::from_ne_bytes(*data);
        if  num == number {
            let value = (number+1).to_ne_bytes();
            let buf = IoSlice::new(&value);
            let r_iov = RemoteIoVec {
                base: *address,
                len: 4
            };
            process_vm_writev(pid,  &[buf], &[r_iov]).unwrap();
        }
    }

    println!("Values after manipulation of memory: (heap {}) (stack {})", string_, number);
}
