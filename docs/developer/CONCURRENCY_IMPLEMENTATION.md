# DebOS Advanced Concurrency Implementation Plan

> **Status:** Phase 4 - Independent (Can run in parallel with other phases)  
> **Priority:** Core Differentiator  
> **Estimated Effort:** 8-12 weeks  
> **Goal:** Make DebOS the most capable multi-processing and multi-threaded OS ever built

---

## Table of Contents

1. [Vision](#vision)
2. [Architecture Overview](#architecture-overview)
3. [Green Threading Model](#green-threading-model)
4. [Work-Stealing Scheduler](#work-stealing-scheduler)
5. [GPU Compute Integration](#gpu-compute-integration)
6. [Async I/O Subsystem](#async-io-subsystem)
7. [Implementation Phases](#implementation-phases)
8. [API Reference](#api-reference)
9. [Performance Targets](#performance-targets)

---

## Vision

DebOS aims to be the **most capable concurrent computing OS** by providing:

1. **Green Threading as a First-Class Citizen**
   - Millions of lightweight threads with minimal overhead
   - M:N threading model (M green threads on N kernel threads)
   - Sub-microsecond context switch times

2. **Work-Stealing Scheduler**
   - Automatic load balancing across all CPU cores
   - NUMA-aware thread placement
   - Lock-free scheduling algorithms

3. **Unified CPU/GPU Compute Model**
   - Seamless offloading to GPU compute units
   - Unified memory addressing
   - Automatic workload partitioning

4. **Zero-Cost Async I/O**
   - io_uring-style completion-based I/O
   - No syscall overhead for batched operations
   - Integrated with green threading

### Why This Matters

| Traditional OS | DebOS |
|---------------|-------|
| ~10,000 threads max | Millions of green threads |
| ~1-10μs context switch | ~50-100ns context switch |
| Manual thread pool management | Automatic work-stealing |
| Separate CPU/GPU programming | Unified compute model |
| Blocking I/O or manual async | Built-in async everywhere |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        User Application                                  │
│              spawn_green!  |  async/await  |  parallel_for!             │
├─────────────────────────────────────────────────────────────────────────┤
│                         libdebos Runtime                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐  │
│  │ Green Thread │  │   Channel    │  │   Future     │  │   Compute   │  │
│  │   Spawner    │  │  (MPMC/SPSC) │  │   Executor   │  │  Dispatch   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────────────────────────┤
│                      Kernel Scheduler (DeK)                              │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                   Work-Stealing Scheduler                        │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │    │
│  │  │ Core 0  │  │ Core 1  │  │ Core 2  │  │ Core N  │  ...       │    │
│  │  │ RunQueue│  │ RunQueue│  │ RunQueue│  │ RunQueue│            │    │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘            │    │
│  │       │ steal ←────┴────────────┴────────────┘                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────────────┤
│                         Hardware Layer                                   │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐    │
│  │         CPU Cores            │  │       GPU Compute Units       │    │
│  │   Core 0 | Core 1 | Core N   │  │    CU 0 | CU 1 | ... | CU M   │    │
│  └──────────────────────────────┘  └──────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Green Threading Model

### What are Green Threads?

Green threads are **lightweight, user-space threads** managed by the runtime rather than the OS kernel. They provide:

- **Minimal memory footprint**: ~2KB stack vs ~2MB for OS threads
- **Fast context switching**: No kernel transition required
- **Massive scalability**: Support millions of concurrent tasks

### M:N Threading Model

```
         User Space (Green Threads)              Kernel Space
    ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐
    │ G1 │ │ G2 │ │ G3 │ │ G4 │ │ G5 │  ... (Millions)
    └──┬─┘ └──┬─┘ └──┬─┘ └──┬─┘ └──┬─┘
       │      │      │      │      │
       └──────┴──────┼──────┴──────┘
                     │
              ┌──────┴──────┐
              │  Scheduler  │
              └──────┬──────┘
                     │
       ┌─────────────┼─────────────┐
       │             │             │
    ┌──┴──┐      ┌──┴──┐      ┌──┴──┐
    │ KT1 │      │ KT2 │      │ KT3 │   (N = CPU cores)
    └─────┘      └─────┘      └─────┘
       │             │             │
    [Core 0]     [Core 1]     [Core 2]
```

### Green Thread Structure

```rust
/// A green thread (fiber/coroutine)
pub struct GreenThread {
    /// Unique thread ID
    pub id: GreenThreadId,
    
    /// Thread state
    pub state: GreenThreadState,
    
    /// Lightweight stack (growable, starts at 2KB)
    pub stack: GrowableStack,
    
    /// Saved CPU context (minimal - just callee-saved registers)
    pub context: GreenContext,
    
    /// Priority (0-255, higher = more important)
    pub priority: u8,
    
    /// CPU affinity hint (None = any core)
    pub affinity: Option<CoreId>,
    
    /// Parent thread (for structured concurrency)
    pub parent: Option<GreenThreadId>,
    
    /// Join handle for waiting on completion
    pub join_handle: Option<JoinHandle>,
    
    /// Local storage
    pub local_storage: LocalStorage,
}

/// Minimal context for green thread switching
#[repr(C)]
pub struct GreenContext {
    // Only callee-saved registers (much smaller than full context)
    #[cfg(target_arch = "x86_64")]
    pub rbx: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rsp: u64,  // Stack pointer
    pub rip: u64,  // Return address
    
    #[cfg(target_arch = "aarch64")]
    pub x19_x29: [u64; 11],  // Callee-saved registers
    pub sp: u64,
    pub lr: u64,
}

/// Green thread states
pub enum GreenThreadState {
    /// Ready to run
    Ready,
    /// Currently executing
    Running,
    /// Waiting on I/O
    IoBlocked(IoHandle),
    /// Waiting on channel
    ChannelBlocked(ChannelId),
    /// Waiting for child threads
    JoinBlocked(Vec<GreenThreadId>),
    /// Sleeping until timestamp
    Sleeping(Timestamp),
    /// Finished execution
    Completed(Result<(), Error>),
}
```

### Growable Stacks

Green threads use **segmented/growable stacks** to minimize memory usage:

```rust
/// Growable stack that starts small and expands on demand
pub struct GrowableStack {
    /// Base address of stack
    base: *mut u8,
    
    /// Current stack size
    size: usize,
    
    /// Maximum allowed size
    max_size: usize,
    
    /// Guard page location
    guard_page: *mut u8,
}

impl GrowableStack {
    /// Initial stack size (2KB)
    const INITIAL_SIZE: usize = 2 * 1024;
    
    /// Stack growth increment
    const GROWTH_SIZE: usize = 4 * 1024;
    
    /// Maximum stack size (1MB)
    const MAX_SIZE: usize = 1024 * 1024;
    
    /// Create a new growable stack
    pub fn new() -> Self {
        // Allocate initial small stack with guard page
        let base = unsafe { 
            alloc::alloc::alloc_zeroed(
                Layout::from_size_align(Self::INITIAL_SIZE, 4096).unwrap()
            )
        };
        
        GrowableStack {
            base,
            size: Self::INITIAL_SIZE,
            max_size: Self::MAX_SIZE,
            guard_page: base, // Guard at bottom
        }
    }
    
    /// Handle stack overflow by growing
    pub fn grow(&mut self) -> Result<(), StackError> {
        if self.size >= self.max_size {
            return Err(StackError::Overflow);
        }
        
        let new_size = core::cmp::min(
            self.size + Self::GROWTH_SIZE,
            self.max_size
        );
        
        // Reallocate with larger size
        // (In practice, use virtual memory tricks)
        self.size = new_size;
        Ok(())
    }
}
```

### Context Switching (Ultra-Fast)

```rust
// x86_64 green thread context switch (~50 cycles)
#[cfg(target_arch = "x86_64")]
#[naked]
pub unsafe extern "C" fn green_switch(
    old: *mut GreenContext,
    new: *const GreenContext
) {
    naked_asm!(
        // Save callee-saved registers to old context
        "mov [rdi + 0x00], rbx",
        "mov [rdi + 0x08], rbp",
        "mov [rdi + 0x10], r12",
        "mov [rdi + 0x18], r13",
        "mov [rdi + 0x20], r14",
        "mov [rdi + 0x28], r15",
        "mov [rdi + 0x30], rsp",
        "lea rax, [rip + 1f]",      // Return address
        "mov [rdi + 0x38], rax",
        
        // Load callee-saved registers from new context
        "mov rbx, [rsi + 0x00]",
        "mov rbp, [rsi + 0x08]",
        "mov r12, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r14, [rsi + 0x20]",
        "mov r15, [rsi + 0x28]",
        "mov rsp, [rsi + 0x30]",
        "jmp [rsi + 0x38]",         // Jump to new thread
        
        "1:",                        // Return point
        "ret",
    );
}

// AArch64 green thread context switch
#[cfg(target_arch = "aarch64")]
#[naked]
pub unsafe extern "C" fn green_switch(
    old: *mut GreenContext,
    new: *const GreenContext
) {
    naked_asm!(
        // Save callee-saved registers (x19-x29, sp, lr)
        "stp x19, x20, [x0, #0]",
        "stp x21, x22, [x0, #16]",
        "stp x23, x24, [x0, #32]",
        "stp x25, x26, [x0, #48]",
        "stp x27, x28, [x0, #64]",
        "stp x29, x30, [x0, #80]",   // x30 = lr
        "mov x9, sp",
        "str x9, [x0, #96]",
        
        // Load callee-saved registers
        "ldp x19, x20, [x1, #0]",
        "ldp x21, x22, [x1, #16]",
        "ldp x23, x24, [x1, #32]",
        "ldp x25, x26, [x1, #48]",
        "ldp x27, x28, [x1, #64]",
        "ldp x29, x30, [x1, #80]",
        "ldr x9, [x1, #96]",
        "mov sp, x9",
        
        "ret",                       // Return to new thread (via lr)
    );
}
```

---

## Work-Stealing Scheduler

The work-stealing scheduler ensures all CPU cores are utilized efficiently.

### Design

```
              ┌─────────────────────────────────────────────────┐
              │            Global Injection Queue               │
              │     (for newly spawned threads from I/O)        │
              └─────────────────────┬───────────────────────────┘
                                    │ distribute
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
          ▼                         ▼                         ▼
    ┌──────────────┐         ┌──────────────┐         ┌──────────────┐
    │   Core 0     │  steal  │   Core 1     │  steal  │   Core 2     │
    │ ┌──────────┐ │ ◄─────► │ ┌──────────┐ │ ◄─────► │ ┌──────────┐ │
    │ │ RunQueue │ │         │ │ RunQueue │ │         │ │ RunQueue │ │
    │ │ (lock-   │ │         │ │ (lock-   │ │         │ │ (lock-   │ │
    │ │  free)   │ │         │ │  free)   │ │         │ │  free)   │ │
    │ └──────────┘ │         │ └──────────┘ │         │ └──────────┘ │
    │              │         │              │         │              │
    │ [Executor]   │         │ [Executor]   │         │ [Executor]   │
    └──────────────┘         └──────────────┘         └──────────────┘
```

### Lock-Free Work-Stealing Deque

Each core has a double-ended queue (deque) that supports:
- **Push/Pop** from the local end (owner only)
- **Steal** from the remote end (other cores)

```rust
/// Lock-free work-stealing deque (Chase-Lev algorithm)
pub struct WorkStealingDeque<T> {
    /// Array of tasks (power of 2 size)
    buffer: AtomicPtr<T>,
    
    /// Capacity (power of 2)
    capacity: usize,
    
    /// Bottom index (owner writes)
    bottom: AtomicUsize,
    
    /// Top index (stealers read/CAS)
    top: AtomicUsize,
}

impl<T> WorkStealingDeque<T> {
    /// Push a task (owner only, fast path)
    pub fn push(&self, task: T) {
        let bottom = self.bottom.load(Ordering::Relaxed);
        let top = self.top.load(Ordering::Acquire);
        
        // Grow if needed
        if bottom - top >= self.capacity - 1 {
            self.grow();
        }
        
        unsafe {
            let buffer = self.buffer.load(Ordering::Relaxed);
            (*buffer.add(bottom % self.capacity)) = task;
        }
        
        // Release barrier ensures task is visible before bottom update
        self.bottom.store(bottom + 1, Ordering::Release);
    }
    
    /// Pop a task (owner only)
    pub fn pop(&self) -> Option<T> {
        let bottom = self.bottom.load(Ordering::Relaxed) - 1;
        self.bottom.store(bottom, Ordering::Relaxed);
        
        // Full barrier
        core::sync::atomic::fence(Ordering::SeqCst);
        
        let top = self.top.load(Ordering::Relaxed);
        
        if top <= bottom {
            // Non-empty
            let task = unsafe {
                let buffer = self.buffer.load(Ordering::Relaxed);
                core::ptr::read(buffer.add(bottom % self.capacity))
            };
            
            if top == bottom {
                // Last element, race with stealers
                if self.top.compare_exchange(
                    top, top + 1,
                    Ordering::SeqCst, Ordering::Relaxed
                ).is_err() {
                    // Lost race, queue is empty
                    self.bottom.store(top + 1, Ordering::Relaxed);
                    return None;
                }
                self.bottom.store(top + 1, Ordering::Relaxed);
            }
            
            Some(task)
        } else {
            // Empty
            self.bottom.store(top, Ordering::Relaxed);
            None
        }
    }
    
    /// Steal a task (other cores)
    pub fn steal(&self) -> Option<T> {
        loop {
            let top = self.top.load(Ordering::Acquire);
            
            // Load-load barrier
            core::sync::atomic::fence(Ordering::SeqCst);
            
            let bottom = self.bottom.load(Ordering::Acquire);
            
            if top >= bottom {
                return None; // Empty
            }
            
            let task = unsafe {
                let buffer = self.buffer.load(Ordering::Relaxed);
                core::ptr::read(buffer.add(top % self.capacity))
            };
            
            // Try to claim the task
            if self.top.compare_exchange(
                top, top + 1,
                Ordering::SeqCst, Ordering::Relaxed
            ).is_ok() {
                return Some(task);
            }
            // CAS failed, retry
        }
    }
}
```

### Per-Core Executor

```rust
/// Per-core executor that runs green threads
pub struct CoreExecutor {
    /// Core ID
    core_id: CoreId,
    
    /// Local run queue (work-stealing deque)
    local_queue: WorkStealingDeque<GreenThreadId>,
    
    /// Currently running green thread
    current: Option<GreenThreadId>,
    
    /// Idle state (for power management)
    idle: AtomicBool,
    
    /// Reference to other executors (for stealing)
    siblings: Vec<Arc<CoreExecutor>>,
    
    /// Statistics
    stats: ExecutorStats,
}

impl CoreExecutor {
    /// Main executor loop
    pub fn run(&mut self) -> ! {
        loop {
            // 1. Try to get work from local queue
            if let Some(thread_id) = self.local_queue.pop() {
                self.run_thread(thread_id);
                continue;
            }
            
            // 2. Try to steal from siblings
            if let Some(thread_id) = self.try_steal() {
                self.run_thread(thread_id);
                continue;
            }
            
            // 3. Check global injection queue
            if let Some(thread_id) = self.check_global_queue() {
                self.run_thread(thread_id);
                continue;
            }
            
            // 4. Go idle (power-efficient wait)
            self.idle_wait();
        }
    }
    
    /// Try to steal work from other cores
    fn try_steal(&mut self) -> Option<GreenThreadId> {
        // Random starting point to avoid contention
        let start = self.random_core();
        
        for i in 0..self.siblings.len() {
            let idx = (start + i) % self.siblings.len();
            if idx == self.core_id.0 { continue; }
            
            if let Some(task) = self.siblings[idx].local_queue.steal() {
                self.stats.stolen += 1;
                return Some(task);
            }
        }
        
        None
    }
    
    /// Run a green thread until it yields/blocks
    fn run_thread(&mut self, thread_id: GreenThreadId) {
        let thread = get_green_thread(thread_id);
        self.current = Some(thread_id);
        
        // Switch to thread
        unsafe {
            green_switch(&mut self.executor_context, &thread.context);
        }
        
        // Thread yielded back
        self.current = None;
        
        // Handle thread state
        match thread.state {
            GreenThreadState::Ready => {
                // Re-queue for later
                self.local_queue.push(thread_id);
            }
            GreenThreadState::IoBlocked(_) => {
                // Register with I/O subsystem
                register_io_waiter(thread_id);
            }
            GreenThreadState::Completed(_) => {
                // Wake up joiners, cleanup
                complete_thread(thread_id);
            }
            // ... handle other states
        }
    }
}
```

### NUMA Awareness

```rust
/// NUMA topology information
pub struct NumaTopology {
    /// Number of NUMA nodes
    num_nodes: usize,
    
    /// Cores per node
    cores_per_node: Vec<Vec<CoreId>>,
    
    /// Distance matrix between nodes
    distances: Vec<Vec<u8>>,
}

/// NUMA-aware thread placement
impl CoreExecutor {
    /// Prefer local NUMA node for stealing
    fn try_steal_numa_aware(&mut self) -> Option<GreenThreadId> {
        let my_node = numa_node_for_core(self.core_id);
        
        // First, try cores in same NUMA node
        for &core in &NUMA_TOPOLOGY.cores_per_node[my_node] {
            if core == self.core_id { continue; }
            if let Some(task) = self.siblings[core.0].local_queue.steal() {
                return Some(task);
            }
        }
        
        // Then try remote nodes (sorted by distance)
        for node in nodes_by_distance(my_node) {
            for &core in &NUMA_TOPOLOGY.cores_per_node[node] {
                if let Some(task) = self.siblings[core.0].local_queue.steal() {
                    return Some(task);
                }
            }
        }
        
        None
    }
}
```

---

## GPU Compute Integration

### Unified Compute Model

DebOS treats GPU compute units as first-class execution resources:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Unified Scheduler                             │
│                                                                  │
│  ┌────────────────────────┐    ┌────────────────────────────┐   │
│  │      CPU Executors     │    │      GPU Executors         │   │
│  │  ┌────┐ ┌────┐ ┌────┐  │    │  ┌────┐ ┌────┐ ┌────┐     │   │
│  │  │ C0 │ │ C1 │ │ C2 │  │◄──►│  │ G0 │ │ G1 │ │ G2 │ ... │   │
│  │  └────┘ └────┘ └────┘  │    │  └────┘ └────┘ └────┘     │   │
│  └────────────────────────┘    └────────────────────────────┘   │
│                                                                  │
│                    Unified Memory (UMA)                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │   Accessible from both CPU and GPU                        │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### GPU Compute Tasks

```rust
/// A GPU compute task
pub struct GpuTask {
    /// Task ID
    id: GpuTaskId,
    
    /// Compute kernel to execute
    kernel: GpuKernel,
    
    /// Input data (in unified memory)
    inputs: Vec<GpuBuffer>,
    
    /// Output buffers
    outputs: Vec<GpuBuffer>,
    
    /// Grid dimensions
    grid: (u32, u32, u32),
    
    /// Block dimensions
    block: (u32, u32, u32),
    
    /// Completion callback
    on_complete: Option<Box<dyn FnOnce(GpuResult)>>,
}

/// Submit work to GPU
pub fn gpu_submit(task: GpuTask) -> GpuFuture {
    let future = GpuFuture::new();
    
    GPU_SCHEDULER.lock().submit(task, future.clone());
    
    future
}

/// GPU compute kernel (compiled shader/compute code)
pub struct GpuKernel {
    /// Platform-specific kernel binary
    #[cfg(feature = "metal")]
    metal_function: MetalFunction,
    
    #[cfg(feature = "vulkan")]
    vulkan_shader: VulkanShader,
    
    /// Uniform/constant data
    uniforms: Vec<u8>,
}
```

### Automatic CPU/GPU Partitioning

```rust
/// Automatically partition work between CPU and GPU
pub fn parallel_compute<T, F>(
    data: &mut [T],
    f: F,
) -> ComputeResult
where
    F: Fn(&mut T) + Sync + Send + GpuCompatible,
{
    let len = data.len();
    
    // Heuristic: use GPU for large datasets
    if len > GPU_THRESHOLD && is_gpu_available() {
        let (cpu_portion, gpu_portion) = partition_for_hardware(len);
        
        // Spawn CPU work
        let cpu_handle = spawn_green(|| {
            data[..cpu_portion].par_iter_mut().for_each(&f);
        });
        
        // Submit GPU work
        let gpu_future = gpu_submit(GpuTask {
            kernel: f.to_gpu_kernel(),
            inputs: vec![GpuBuffer::from(&data[cpu_portion..])],
            ..Default::default()
        });
        
        // Wait for both
        cpu_handle.join();
        gpu_future.await;
    } else {
        // CPU only
        data.par_iter_mut().for_each(&f);
    }
    
    ComputeResult::Ok
}
```

---

## Async I/O Subsystem

### io_uring-Style Completion I/O

```rust
/// Submission queue entry
#[repr(C)]
pub struct IoSqe {
    /// Operation type
    opcode: IoOpcode,
    /// Flags
    flags: u8,
    /// File descriptor
    fd: i32,
    /// Buffer address
    addr: u64,
    /// Length
    len: u32,
    /// Offset
    offset: u64,
    /// User data (returned in completion)
    user_data: u64,
}

/// Completion queue entry
#[repr(C)]
pub struct IoCqe {
    /// User data from submission
    user_data: u64,
    /// Result (bytes transferred or error)
    result: i32,
    /// Flags
    flags: u32,
}

/// IO Ring for batched async I/O
pub struct IoRing {
    /// Submission queue
    sq: SubmissionQueue,
    /// Completion queue
    cq: CompletionQueue,
    /// Shared memory between user/kernel
    ring_mem: MappedMemory,
}

impl IoRing {
    /// Submit a batch of I/O operations (no syscall if queue has space)
    pub fn submit(&mut self, ops: &[IoSqe]) -> Result<usize, IoError> {
        for op in ops {
            self.sq.push(*op)?;
        }
        
        // Only syscall if kernel needs wake-up
        if self.sq.needs_wakeup() {
            sys_io_submit(self.ring_fd)?;
        }
        
        Ok(ops.len())
    }
    
    /// Poll for completions (no syscall)
    pub fn poll(&mut self) -> impl Iterator<Item = IoCqe> {
        self.cq.drain()
    }
    
    /// Wait for completions (blocking)
    pub fn wait(&mut self, min_complete: usize) -> Result<usize, IoError> {
        while self.cq.len() < min_complete {
            sys_io_wait(self.ring_fd, min_complete)?;
        }
        Ok(self.cq.len())
    }
}
```

### Integration with Green Threads

```rust
/// Async file operations using green threads
impl AsyncFile {
    /// Read asynchronously (green thread yields until complete)
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        // Get current green thread
        let thread_id = current_green_thread();
        
        // Submit I/O
        let sqe = IoSqe {
            opcode: IoOpcode::Read,
            fd: self.fd,
            addr: buf.as_mut_ptr() as u64,
            len: buf.len() as u32,
            user_data: thread_id.0 as u64,
            ..Default::default()
        };
        
        IO_RING.lock().submit(&[sqe])?;
        
        // Yield green thread
        yield_green_thread(GreenThreadState::IoBlocked(self.fd));
        
        // When we resume, I/O is complete
        get_io_result(thread_id)
    }
}

/// I/O completion handler (runs on I/O thread)
fn io_completion_handler() {
    loop {
        for cqe in IO_RING.lock().poll() {
            let thread_id = GreenThreadId(cqe.user_data as usize);
            
            // Store result
            set_io_result(thread_id, cqe.result);
            
            // Wake up green thread
            wake_green_thread(thread_id);
        }
        
        // Wait for more completions
        IO_RING.lock().wait(1).unwrap();
    }
}
```

---

## Implementation Phases

### Phase 4A: Green Threading Core (Week 1-3)

**Goal:** Basic green thread spawning and cooperative switching

- [ ] `GreenThread` structure with minimal context
- [ ] `GreenContext` save/restore for x86_64 and AArch64
- [ ] `GrowableStack` with guard pages
- [ ] Basic spawn/yield/exit operations
- [ ] Single-threaded executor for testing

**Deliverables:**
```
kernel/src/green/
├── mod.rs          # Core types and API
├── thread.rs       # GreenThread implementation
├── context.rs      # Context switching
├── stack.rs        # Growable stacks
└── executor.rs     # Single-core executor
```

### Phase 4B: Work-Stealing Scheduler (Week 4-6)

**Goal:** Multi-core work-stealing with NUMA awareness

- [ ] Lock-free work-stealing deque
- [ ] Per-core executors
- [ ] Work stealing algorithm
- [ ] NUMA topology detection
- [ ] NUMA-aware stealing

**Deliverables:**
```
kernel/src/green/
├── deque.rs        # Lock-free deque
├── scheduler.rs    # Work-stealing scheduler
├── numa.rs         # NUMA awareness
└── stats.rs        # Performance statistics
```

### Phase 4C: Async I/O Integration (Week 7-8)

**Goal:** io_uring-style async I/O with green thread integration

- [ ] IoRing submission/completion queues
- [ ] Kernel-side I/O handling
- [ ] Green thread I/O blocking
- [ ] Async file/socket operations

**Deliverables:**
```
kernel/src/io/
├── ring.rs         # IoRing implementation
├── ops.rs          # I/O operations
└── async.rs        # Async integration

libdebos/src/
├── async_io.rs     # Async I/O API
└── async_net.rs    # Async networking
```

### Phase 4D: GPU Compute (Week 9-12)

**Goal:** Unified CPU/GPU compute model

- [ ] GPU device enumeration
- [ ] Unified memory management
- [ ] GPU task submission
- [ ] CPU/GPU work partitioning
- [ ] Metal backend (Apple Silicon)
- [ ] Vulkan compute backend (optional)

**Deliverables:**
```
kernel/src/gpu/
├── mod.rs          # GPU subsystem
├── device.rs       # Device management
├── memory.rs       # Unified memory
├── scheduler.rs    # GPU task scheduler
├── metal.rs        # Metal backend
└── vulkan.rs       # Vulkan backend
```

---

## API Reference

### User-Space API (libdebos)

```rust
// ===== Green Thread Spawning =====

/// Spawn a new green thread
pub fn spawn_green<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static;

/// Spawn with options
pub fn spawn_green_with<F, T>(opts: SpawnOptions, f: F) -> JoinHandle<T>;

/// Spawn options
pub struct SpawnOptions {
    pub name: Option<String>,
    pub priority: u8,
    pub stack_size: usize,
    pub affinity: Option<CoreId>,
}

// ===== Yielding =====

/// Yield current green thread
pub fn yield_now();

/// Sleep for duration
pub async fn sleep(duration: Duration);

// ===== Channels (MPMC) =====

/// Create a bounded channel
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>);

/// Create an unbounded channel
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>);

// ===== Parallel Iterators =====

/// Parallel iterator trait
pub trait ParallelIterator: Iterator {
    fn par_for_each<F>(self, f: F)
    where
        F: Fn(Self::Item) + Sync + Send;
    
    fn par_map<B, F>(self, f: F) -> ParMap<Self, F>
    where
        F: Fn(Self::Item) -> B + Sync + Send;
}

// ===== Structured Concurrency =====

/// Scope for structured concurrency
pub fn scope<'env, F, T>(f: F) -> T
where
    F: FnOnce(&Scope<'env>) -> T;

impl<'env> Scope<'env> {
    /// Spawn a scoped green thread
    pub fn spawn<F, T>(&self, f: F) -> ScopedJoinHandle<'env, T>
    where
        F: FnOnce() -> T + Send + 'env,
        T: Send + 'env;
}

// ===== GPU Compute =====

/// Submit GPU compute task
pub fn gpu_compute<F>(f: F) -> GpuFuture
where
    F: GpuKernel;

/// Parallel compute (auto CPU/GPU partitioning)
pub fn parallel_compute<T, F>(data: &mut [T], f: F)
where
    F: Fn(&mut T) + Sync + Send + GpuCompatible;
```

### Usage Examples

```rust
use libdebos::green::*;
use libdebos::channel;

// Basic green thread spawning
fn example_spawn() {
    let handle = spawn_green(|| {
        println!("Hello from green thread!");
        42
    });
    
    let result = handle.join().unwrap();
    assert_eq!(result, 42);
}

// Parallel processing with work-stealing
fn example_parallel() {
    let data: Vec<u64> = (0..1_000_000).collect();
    
    let sum: u64 = data.par_iter()
        .map(|x| x * 2)
        .sum();
    
    println!("Sum: {}", sum);
}

// Channel communication
fn example_channels() {
    let (tx, rx) = channel::<u64>(100);
    
    // Producer
    spawn_green(move || {
        for i in 0..1000 {
            tx.send(i).unwrap();
        }
    });
    
    // Consumer
    spawn_green(move || {
        while let Ok(value) = rx.recv() {
            println!("Received: {}", value);
        }
    });
}

// Structured concurrency (all threads complete before scope exits)
fn example_scoped() {
    let mut data = vec![1, 2, 3, 4, 5];
    
    scope(|s| {
        for item in &mut data {
            s.spawn(move || {
                *item *= 2;
            });
        }
    }); // All threads complete here
    
    assert_eq!(data, vec![2, 4, 6, 8, 10]);
}

// Async I/O with green threads
async fn example_async_io() {
    let file = AsyncFile::open("data.txt").await?;
    let mut buf = vec![0u8; 4096];
    
    // This yields the green thread, doesn't block OS thread
    let bytes = file.read(&mut buf).await?;
    
    println!("Read {} bytes", bytes);
}

// GPU compute
fn example_gpu() {
    let mut data: Vec<f32> = (0..1_000_000).map(|i| i as f32).collect();
    
    // Automatically uses GPU if available and beneficial
    parallel_compute(&mut data, |x| {
        *x = (*x).sqrt() * 2.0;
    });
}
```

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Green thread spawn | < 100ns | Lock-free allocation |
| Context switch | < 100ns | Only callee-saved registers |
| Memory per thread | ~2KB | Growable stacks |
| Max green threads | 10M+ | Limited only by memory |
| Work steal latency | < 500ns | Lock-free deque |
| I/O submission | < 50ns | No syscall when possible |
| I/O completion | < 1μs | Direct wake-up |
| GPU task submit | < 10μs | Batched commands |

### Benchmark Comparisons

| Operation | Linux pthread | Go goroutine | DebOS green |
|-----------|--------------|--------------|-------------|
| Spawn | ~20μs | ~300ns | **< 100ns** |
| Context switch | ~1.5μs | ~200ns | **< 100ns** |
| Stack memory | 2MB | 2KB | **2KB** |
| Max concurrent | ~10K | ~100K | **10M+** |

---

## References

- Go runtime scheduler (work-stealing design)
- Tokio (Rust async runtime)
- io_uring (Linux async I/O)
- Grand Central Dispatch (Apple)
- Intel TBB (work-stealing)
- Cilk (structured parallelism)

---

*Document Version: 1.0.0*  
*Phase: 4 (Independent)*  
*Last Updated: November 2024*

