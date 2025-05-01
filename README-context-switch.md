# Contex Switching between Threads

These are some notes on how the context switching between threads works 
in `c_fibres`.

```
t_yield():	Current thread: 0, switching from thread 0 to thread 1, thread 0 sp: 0x0000000000000000, thread 1 sp: 0x00007f8778407fe0

->  0x100002dfb <+827>:  call   0x1000024c4    ; switch
    0x100002e00 <+832>:  cmp    qword ptr [rbp - 0x1e0], 0x0

(lldb) reg r rsp
     rsp = 0x000000030419c778 
      ^
      |
      stack pointer of thread 0

(lldb) mem read 0x000000030419c778
0x30419c778: 00 2e 00 00 01 00 00 00 07 00 00 00 00 00 00 00  ................
             ^
             |
             return address of thread 0 at the top of the stack

c_fibres`switch:
->  0x1000024df <+27>: mov    rsp, qword ptr [rsi]

(lldb) reg r rsi
     rsi = 0x00007f8773904200

(lldb) mem read 0x00007f8773904200
0x7f8773904200: e0 7f 40 78 87 7f 00 00 00 00 00 00 00 00 00 00  ..@x............
                ^
                |
                stack pointer of thread 1; we are switching to thread 1
```
At this point, the Thread 1 context is such that stack pointer `0x00007f8778407fe0`, points to the address of the function `f()`.
Hence, `ret` from `switch` will jump to the beginning of `f()`:

```
(lldb) mem read 0x00007f8778407fe0
0x7f8778407fe0: 90 34 00 00 01 00 00 00 c0 24 00 00 01 00 00 00  .4.......$......
0x7f8778407ff0: b0 32 00 00 01 00 00 00 00 00 00 00 00 00 00 00  .2..............
```

We can see that the first 8 bytes at `0x00007f8778407fe0` are 0x0000000100003490.  The next 8 bytes, `0x00000001000024c0`
must be the address of the `skip` function and the following 8 bytes, `0x00000001000032b0` must be the address of the `guard` function.

```
c_fibres`c_fibres::f::ha2df35a67ab47220:
->  0x100003490 <+0>:   push   rbp
       ^
       | 
       start of function f

(lldb) reg r rsp
     rsp = 0x00007fb390407fe8
(lldb) mem read 0x00007fb390407fe8
0x7fb390407fe8: c0 24 00 00 01 00 00 00 b0 32 00 00 01 00 00 00  .$.......2......
```

Note that the top of the stack now contains the address of the `skip` function, `0x00000001000024c0`.
We are now entering the function `f`, and the compiler will generate normal function prologue and epilogue code.

```
c_fibres`c_fibres::f::ha2df35a67ab47220:
->  0x100003490 <+0>:   push   rbp
    0x100003491 <+1>:   mov    rbp, rsp
    0x100003494 <+4>:   sub    rsp, 0x110

   330 	fn f() {
-> 331 	    println!("\t\tf():\t\tThread: 1 Starting");

(lldb) reg r rsp
     rsp = 0x00007f8778407ed0
```

We are about to call `yield_thread()` from Thread 1:

```
-> 335 	        yield_thread();
   336 	    }
   337 	    println!("\t\tf():\t\tThread 1 Finished");
   338 	}

->  0x100003599 <+265>: call   0x1000033a0    ; c_fibres::yield_thread::h3572076e2f0f57b3 at c_fibres.rs:289
    0x10000359e <+270>: jmp    0x100003504    ; <+116> at c_fibres.rs:333:14

Target 0: (c_fibres) stopped.
(lldb) reg r rsp
     rsp = 0x00007f8778407ed0

(lldb) si

-> 289 	pub fn yield_thread() {
   290 	    unsafe {
   291 	        let rt_ptr = RUNTIME as *mut Runtime;
   292 	        let _current = rt_ptr.as_ref().unwrap().current();
Target 0: (c_fibres) stopped.
(lldb) reg r rsp
     rsp = 0x00007fb390407ec8

(lldb) mem read 0x00007fb390407ec8
0x7fb390407ec8: 9e 35 00 00 01 00 00 00 00 00 00 00 00 00 00 00  .5..............
                ^
                |
              return address of Thread 1 after calling `yield_thread()`.
              which as we can see above is line 333 in `c_fibres.rs`.
```

Since `yield_thread()` is a normal function, the compiler will generate normal
function prologue and epilogue code:

```
c_fibres`c_fibres::yield_thread::h3572076e2f0f57b3:
->  0x1000033a0 <+0>:   push   rbp
    0x1000033a1 <+1>:   mov    rbp, rsp
    0x1000033a4 <+4>:   sub    rsp, 0x80
```

We are about to call `t_yield()` from `yield_thread()`:

```
   291 	        let rt_ptr = RUNTIME as *mut Runtime;
   292 	        let _current = rt_ptr.as_ref().unwrap().current();
   293 	        println!("\t\tyield_thread():\tCurrent thread: {}", _current);
-> 294 	        (*rt_ptr).t_yield();

->  0x100003475 <+213>: call   0x100002ac0    ; c_fibres::Runtime::t_yield::h339e82c5cd9443e2 at c_fibres.rs:179
    0x10000347a <+218>: add    rsp, 0x80
```

We are now about to enter the `t_yield()` function from Thread 1.

```
-> 179 	    fn t_yield(&mut self) -> bool {
   180 	        let mut pos = self.current();

(lldb) reg r rsp
     rsp = 0x00007fb390407e38
(lldb) mem read 0x00007fb390407e38
0x7fb390407e38: 7a 34 00 00 01 00 00 00 a0 7e 40 90 b3 7f 00 00  z4.......~@.....
                ^
                |
              return address of Thread 1 after calling `t_yield()`.
              We can see above that the instruction at that address is:
              `add rsp, 0x80`.
```

`t_yield()` is a normal function, so the compiler will generate normal 
function prologue and epilogue code:

```
c_fibres`c_fibres::Runtime::t_yield::h339e82c5cd9443e2:
    0x100002ac0 <+0>:    push   rbp
->  0x100002ac1 <+1>:    mov    rbp, rsp
    0x100002ac4 <+4>:    sub    rsp, 0x280
```

```
t_yield():	Current thread: 1, switching from thread 1 to thread 0, thread 1 sp: 0x00007fb390407fe0, thread 0 sp: 0x000000030419c778
```

We are about to call `switch()` from `t_yield()` from Thread 1.

```
->  0x100002dfb <+827>:  call   0x1000024c4    ; switch
    0x100002e00 <+832>:  cmp    qword ptr [rbp - 0x1e0], 0x0

c_fibres`switch:
->  0x1000024c4 <+0>:  mov    qword ptr [rdi], rsp
    0x1000024c7 <+3>:  mov    qword ptr [rdi + 0x8], r15

(lldb) reg r rsp
     rsp = 0x00007fb390407ba8
(lldb) mem read 0x00007fb390407ba8
0x7fb390407ba8: 00 2e 00 00 01 00 00 00 07 00 00 00 00 00 00 00  ................
                ^
                |
                New location where Thread 1 will jump to after the next 
                context switch.  This is the address right after the 
                call to `switch()` in `t_yield()`.
```

We are about to restore the context of Thread 0:

```
c_fibres`switch:
->  0x1000024df <+27>: mov    rsp, qword ptr [rsi]
    0x1000024e2 <+30>: mov    r15, qword ptr [rsi + 0x8]
    0x1000024e6 <+34>: mov    r14, qword ptr [rsi + 0x10]
    0x1000024ea <+38>: mov    r13, qword ptr [rsi + 0x18]
Target 0: (c_fibres) stopped.
(lldb) reg r rsi
     rsi = 0x00007fb3891041a8
(lldb) mem read 0x00007fb3891041a8
0x7fb3891041a8: 78 c7 19 04 03 00 00 00 00 30 9a 03 03 00 00 00  x........0......
```

Here, we can see that `rsi` points to the new stack pointer of Thread 0, `0x000000030419c778`.
The content of that address will contain the return location of Thread 0, which is
just after the call to `switch` in `t_yield()` from Thread 0.

This will end up uwinding the stack of Thread 0, until it reaches the `run` function
again where it calls `t_yield()` again, and the cycle continues.

```
   179 	    fn t_yield(&mut self) -> bool {
-> 180 	        let mut pos = self.current();

t_yield():	Current thread: 0, switching from thread 0 to thread 1, thread 0 sp: 0x000000030419c778, thread 1 sp: 0x00007fb390407ba8
```

We are about to call `switch()` from `t_yield()` from Thread 0.

```
->  0x100002dfb <+827>:  call   0x1000024c4    ; switch
    0x100002e00 <+832>:  cmp    qword ptr [rbp - 0x1e0], 0x0

c_fibres`switch:
->  0x1000024c4 <+0>:  mov    qword ptr [rdi], rsp

(lldb) reg r rsp
     rsp = 0x000000030419c778
(lldb) mem read 0x000000030419c778
0x30419c778: 00 2e 00 00 01 00 00 00 07 00 00 00 00 00 00 00  ................
              ^
              |
              return address of Thread 0 at the top of the stack
```

We are about to restore the context of Thread 1:

```
c_fibres`switch:
->  0x1000024df <+27>: mov    rsp, qword ptr [rsi]

(lldb) reg r rsi
     rsi = 0x00007fb389104200
(lldb) mem read 0x00007fb389104200
0x7fb389104200: a8 7b 40 90 b3 7f 00 00 00 00 00 00 00 00 00 00  .{@.............

`rsi` contains the address 0x00007fb390407ba8.

(lldb) mem read 0x00007fb390407ba8
0x7fb390407ba8: 00 2e 00 00 01 00 00 00 07 00 00 00 00 00 00 00  ................
```

The address `0x00007bf390407ba8` contains the address `0x0000000100002e00`.  This
is the location right after the call to `switch()` in `t_yield()`.  This is where
Thread 1 will continue executing after the context switch.

```
->  0x100002e00 <+832>:  cmp    qword ptr [rbp - 0x1e0], 0x0
    0x100002e08 <+840>:  je     0x100002e3b    ; <+891> at c_fibres.rs:239:13

The source code is (in function `t_yield`):

-> 238 	        if _current == 0 {
   239 	            println!("\t\tt_yield():\tCurrent thread: {}", _current);
   240 	        } else {
```

We are about to return from `t_yield` from Thread 1:

```
(lldb) reg r rsp
     rsp = 0x00007fb390407e38
(lldb) mem read 0x00007fb390407e38
0x7fb390407e38: 7a 34 00 00 01 00 00 00 a0 7e 40 90 b3 7f 00 00  z4.......~@.....
```

Thread 1 is now going to jump to the address `0x000000010000347a`.

```
->  0x10000347a <+218>: add    rsp, 0x80
    0x100003481 <+225>: pop    rbp
    0x100003482 <+226>: ret

->  0x100003481 <+225>: pop    rbp
    0x100003482 <+226>: ret

(lldb) reg r rsp
     rsp = 0x00007fb390407ec8
(lldb) mem read 0x00007fb390407ec8
0x7fb390407ec8: 9e 35 00 00 01 00 00 00 00 00 00 00 00 00 00 00  .5..............
```

Thread 1 is now going to jump to the address `0x000000010000359e`.

As we saw earlier, this is the location:

```
->  0x10000359e <+270>: jmp    0x100003504    ; <+116> at c_fibres.rs:333:14
    0x1000035a3 <+275>: lea    rdi, [rbp - 0x30]
```

Thread 1 now continues it's next iteration:

```
-> 333 	    for i in 0..=1 {
   334 	        println!("\t\tf():\t\tThread: {} counter: {}", id, i);
   335 	        yield_thread();
   336 	    }
```

Last iteration of Thread 1:

About to jump to `yield_thread()` from f:

```
->  0x100003599 <+265>: call   0x1000033a0    ; c_fibres::yield_thread::h3572076e2f0f57b3 at c_fibres.rs:289
    0x10000359e <+270>: jmp    0x100003504    ; <+116> at c_fibres.rs:333:14

-> 289 	pub fn yield_thread() {
   290 	    unsafe {
   291 	        let rt_ptr = RUNTIME as *mut Runtime;

(lldb) reg r rsp
     rsp = 0x00007fdc48407ec8
(lldb) mem read 0x00007fdc48407ec8
0x7fdc48407ec8: 9e 35 00 00 01 00 00 00 00 00 00 00 00 00 00 00  .5..............
                ^
                |
                return address of Thread 1

   332 	    let id = 1;
-> 333 	    for i in 0..=1 {
   334 	        println!("\t\tf():\t\tThread: {} counter: {}", id, i);
```

The iteration of Thread 1 is finished:

```
   334 	        println!("\t\tf():\t\tThread: {} counter: {}", id, i);
   335 	        yield_thread();
   336 	    }
-> 337 	    println!("\t\tf():\t\tThread 1 Finished");
```

The disassembly code for the epilogue of the function `f` is:

```
    0x1000035bc <+300>: add    rsp, 0x110
->  0x1000035c3 <+307>: pop    rbp
    0x1000035c4 <+308>: ret
```

After this, rsp is as follows:

```
(lldb) reg r rsp
     rsp = 0x00007fdc48407fe8
(lldb) mem read 0x00007fdc48407fe8
0x7fdc48407fe8: c0 24 00 00 01 00 00 00 b0 32 00 00 01 00 00 00  .$.......2......
```

The top of rsp points to the address of the `skip` function, `0x00000001000024c0`.

Thread 1 jumps to the address `0x00000001000024c0`:

```
c_fibres`c_fibres::skip::hde2741fb3c1e356d:
->  0x1000024c0 <+0>: ret
    0x1000024c1 <+1>: nop    dword ptr [rax]
```

skip just issues a `ret` instruction.  rsp is as follows:

```
Target 0: (c_fibres) stopped.
(lldb) reg r rsp
     rsp = 0x00007fdc48407ff0
(lldb) mem read 0x00007fdc48407ff0
0x7fdc48407ff0: b0 32 00 00 01 00 00 00 00 00 00 00 00 00 00 00  .2..............
```

Hence, Thread 1 will jump to the address `0x00000001000032b0`, which is the address of the `guard` function.

```
-> 275 	fn guard() {
   276 	    unsafe {
   277 	        let rt_ptr = RUNTIME as *mut Runtime;
   278 	        let _current = rt_ptr.as_ref().unwrap().current();
Target 0: (c_fibres) stopped.
(lldb) reg r rsp
     rsp = 0x00007fdc48407ff8
(lldb) mem read 0x00007fdc48407ff8
0x7fdc48407ff8: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
```

guard calls `t_return`:

```
-> 117 	    pub fn t_return(&mut self) {
   118 	        // If the calling thread is the `base_thread`, we won't do anything.
   119 	        // Our runtime wil call `t_yield` for us on the base thread.  If it's
   120 	        // called on a spawned thread, we know it's finished since all threads
Target 0: (c_fibres) stopped.
(lldb) reg r rsp
     rsp = 0x00007fdc48407f68
(lldb) mem read 0x00007fdc48407f68
0x7fdc48407f68: 8a 33 00 00 01 00 00 00 0c 7f 40 48 dc 7f 00 00  .3........@H....
```

`t_return` calls `t_yield`:

```
   179 	    fn t_yield(&mut self) -> bool {
-> 180 	        let mut pos = self.current();
```

Switch from Thread 0 to Thread 1:

```
t_yield():	Current thread: 1, switching from thread 1 to thread 0, thread 1 sp: 0x00007fdc48407ba8, thread 0 sp: 0x000000030419c778


->  0x1000024df <+27>: mov    rsp, qword ptr [rsi]
    0x1000024e2 <+30>: mov    r15, qword ptr [rsi + 0x8]
    0x1000024e6 <+34>: mov    r14, qword ptr [rsi + 0x10]
    0x1000024ea <+38>: mov    r13, qword ptr [rsi + 0x18]
Target 0: (c_fibres) stopped.
(lldb) reg r rsi
     rsi = 0x00007fdc437045d8
(lldb) mem read 0x00007fdc437045d8
0x7fdc437045d8: 78 c7 19 04 03 00 00 00 00 30 9a 03 03 00 00 00  x........0......

Thread 0 will jump to the contents of the address `0x000000030419c778`, which is:

(lldb) mem read 0x000000030419c778
0x30419c778: 00 2e 00 00 01 00 00 00 07 00 00 00 00 00 00 00  ................
```

This is the location in `t_yield()` right after the call to `switch()`.  Next,
Thread 0 will end up in `run` and calls `t_yield()` again:

```
   107 	    pub fn run(&mut self) {
-> 108 	        while self.t_yield() {
```

At this point, no more threads are schedule to run and `t_yield` returns false:

```
t_yield():	Current thread: 0, no thread is ready to run, exiting

   190 	                    to run, exiting",
   191 	                    self.current()
   192 	                );
-> 193 	                return false;
```

Thread 0 will end up in `run` and is about to exit the process:

```
   110 	        }
-> 111 	        println!("\t\trun():\t\tThread {} exiting", self.current());
   112 	        std::process::exit(0);
   113 	    }
```
