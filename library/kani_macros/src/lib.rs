// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;
#[cfg(kani)]
use {
    quote::quote,
    syn::parse::{Parse, ParseStream},
    syn::{parse_macro_input, ItemFn},
};

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Leave the code intact, so it can be easily be edited in an IDE,
    // but outside Kani, this code is likely never called.
    let mut result = TokenStream::new();

    result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[allow(dead_code)]
    //     $item
    // )
}

#[cfg(kani)]
struct ProofOptions {
    schedule: syn::Expr,
}

#[cfg(kani)]
impl Parse for ProofOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        assert_eq!(
            ident, "schedule",
            "Only option `schedule` is allowed for #[kani::proof] on `async` functions."
        );
        let _ = input.parse::<syn::Token![=]>()?;
        let schedule = input.parse::<syn::Expr>()?;
        Ok(ProofOptions { schedule })
    }
}

/// Marks a Kani proof harness
///
/// For async harnesses, this will call [`kani::block_on`] (see its documentation for more information).
#[cfg(kani)]
#[proc_macro_attribute]
pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(item as ItemFn);
    let attrs = fn_item.attrs;
    let vis = fn_item.vis;
    let sig = fn_item.sig;
    let body = fn_item.block;

    let kani_attributes = quote!(
        #[kanitool::proof]
        // no_mangle is a temporary hack to make the function "public" so it gets codegen'd
        #[no_mangle]
    );

    if sig.asyncness.is_none() {
        assert!(attr.is_empty(), "#[kani::proof] only takes arguments for async functions for now");
        // Adds `#[kanitool::proof]` and other attributes
        quote!(
            #kani_attributes
            #(#attrs)*
            #vis #sig #body
        )
        .into()
    } else {
        // For async functions, it translates to a synchronous function that calls `kani::block_on`.
        // Specifically, it translates
        // ```ignore
        // #[kani::async_proof]
        // #[attribute]
        // pub async fn harness() { ... }
        // ```
        // to
        // ```ignore
        // #[kani::proof]
        // #[attribute]
        // pub fn harness() {
        //   async fn harness() { ... }
        //   kani::block_on(harness())
        // }
        // ```
        assert!(
            sig.inputs.is_empty(),
            "#[kani::proof] cannot be applied to async functions that take inputs for now"
        );
        if attr.is_empty() {
            let mut modified_sig = sig.clone();
            modified_sig.asyncness = None;
            let fn_name = &sig.ident;
            let spawn_lib = spawn_code();
            quote!(
                #kani_attributes
                #(#attrs)*
                #vis #modified_sig {
                    #sig #body
                    kani::block_on(#fn_name())
                }
            )
            .into()
        } else {
            let config = parse_macro_input!(attr as ProofOptions);
            let mut modified_sig = sig.clone();
            modified_sig.asyncness = None;
            let fn_name = &sig.ident;
            let spawn_lib = spawn_code();
            let schedule = config.schedule;
            quote!(
                #kani_attributes
                #(#attrs)*
                #vis #modified_sig {
                    #sig #body
                    #spawn_lib
                    spawnable_block_on(#fn_name(), #schedule)
                }
            )
            .into()
        }
    }
}

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn unwind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not kani, we should leave the function alone
    item
}

/// Set Loop unwind limit for proof harnesses
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[cfg(kani)]
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Translate #[kani::unwind(arg)] to #[kanitool::unwind(arg)]
    let insert_string = "#[kanitool::unwind(".to_owned() + &attr.to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

#[cfg(kani)]
fn spawn_code() -> impl quote::ToTokens {
    quote! {
        use std::{
            future::Future,
            pin::Pin,
            task::{Context, RawWaker, RawWakerVTable, Waker},
        };

        /// A very simple executor: it polls the future in a busy loop until completion
        ///
        /// This is intended as a drop-in replacement for `futures::block_on`, which Kani cannot handle.
        /// Whereas a clever executor like `block_on` in `futures` or `tokio` would interact with the OS scheduler
        /// to be woken up when a resource becomes available, this is not supported by Kani.
        /// As a consequence, this function completely ignores the waker infrastructure and just polls the given future in a busy loop.
        pub fn block_on<T>(mut fut: impl Future<Output = T>) -> T {
            let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
            let cx = &mut Context::from_waker(&waker);
            // SAFETY: we shadow the original binding, so it cannot be accessed again for the rest of the scope.
            // This is the same as what the pin_mut! macro in the futures crate does.
            let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
            loop {
                match fut.as_mut().poll(cx) {
                    std::task::Poll::Ready(res) => return res,
                    std::task::Poll::Pending => continue,
                }
            }
        }

        /// A dummy waker, which is needed to call [`Future::poll`]
        const NOOP_RAW_WAKER: RawWaker = {
            #[inline]
            unsafe fn clone_waker(_: *const ()) -> RawWaker {
                NOOP_RAW_WAKER
            }

            #[inline]
            unsafe fn noop(_: *const ()) {}

            RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone_waker, noop, noop, noop))
        };
        static mut EXECUTOR: Scheduler = Scheduler::new();
        const MAX_TASKS: usize = 16;

        type BoxFuture = Pin<Box<dyn Future<Output = ()> + Sync + 'static>>;

        /// Allows to parameterize how the scheduler picks the next task to poll in `spawnable_block_on`
        pub trait SchedulingStrategy {
            /// Picks the next task to be scheduled whenever the scheduler needs to pick a task to run next, and whether it can be assumed that the picked task is still running
            ///
            /// Tasks are numbered `0..num_tasks`.
            /// For example, if pick_task(4) returns (2, true) than it picked the task with index 2 and allows Kani to `assume` that this task is still running.
            /// This is useful if the task is chosen nondeterministicall (`kani::any()`) and allows the verifier to discard useless execution branches (such as polling a completed task again).
            fn pick_task(&mut self, num_tasks: usize) -> (usize, bool);
        }

        impl<F: FnMut(usize) -> usize> SchedulingStrategy for F {
            fn pick_task(&mut self, num_tasks: usize) -> (usize, bool) {
                (self(num_tasks), false)
            }
        }

        /// Keeps cycling through the tasks in a deterministic order
        #[derive(Default)]
        pub struct RoundRobin {
            index: usize,
        }

        impl SchedulingStrategy for RoundRobin {
            fn pick_task(&mut self, num_tasks: usize) -> (usize, bool) {
                self.index = (self.index + 1) % num_tasks;
                (self.index, false)
            }
        }

        /// Picks the next task nondeterministically
        #[derive(Default)]
        pub struct NondeterministicScheduling;

        impl SchedulingStrategy for NondeterministicScheduling {
            #[cfg(kani)]
            fn pick_task(&mut self, num_tasks: usize) -> (usize, bool) {
                let index = kani::any();
                kani::assume(index < num_tasks);
                (index, true)
            }

            #[cfg(not(kani))]
            fn pick_task(&mut self, _num_tasks: usize) -> (usize, bool) {
                panic!("Nondeterministic scheduling is only available when running Kani.")
            }
        }

        pub(crate) struct Scheduler {
            /// Using a Vec instead of an array makes the runtime jump from 40s to almost 10min if using Vec::with_capacity and leads to out of memory with Vec::new (even with 64 GB RAM).
            tasks: [Option<BoxFuture>; MAX_TASKS],
            num_tasks: usize,
            num_running: usize,
        }

        impl Scheduler {
            /// Creates a scheduler with an empty task list
            pub(crate) const fn new() -> Scheduler {
                const INIT: Option<BoxFuture> = None;
                Scheduler { tasks: [INIT; MAX_TASKS], num_tasks: 0, num_running: 0 }
            }

            /// Adds a future to the scheduler's task list, returning a JoinHandle
            #[inline] // to work around linking issue
            pub(crate) fn spawn<F: Future<Output = ()> + Sync + 'static>(&mut self, fut: F) -> JoinHandle {
                let index = self.num_tasks;
                self.tasks[index] = Some(Box::pin(fut));
                self.num_tasks += 1;
                self.num_running += 1;
                JoinHandle { index }
            }

            /// Runs the scheduler with the given scheduling plan until all tasks have completed
            #[inline] // to work around linking issue
            fn run(&mut self, mut scheduling_plan: impl SchedulingStrategy) {
                let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
                let cx = &mut Context::from_waker(&waker);
                while self.num_running > 0 {
                    let (index, can_assume_running) = scheduling_plan.pick_task(self.num_tasks);
                    let task = &mut self.tasks[index];
                    if let Some(fut) = task.as_mut() {
                        match fut.as_mut().poll(cx) {
                            std::task::Poll::Ready(()) => {
                                self.num_running -= 1;
                                let _prev = std::mem::replace(task, None);
                            }
                            std::task::Poll::Pending => (),
                        }
                    } else if can_assume_running {
                        kani::assume(false); // useful so that we can assume that a nondeterministically picked task is still running
                    }
                }
            }

            /// Polls the given future and the tasks it may spawn until all of them complete
            ///
            /// TODO: it would be good if we could error if `spawn` is used inside the given future.
            #[inline] // to work around linking issue
            fn block_on<F: Future<Output = ()> + Sync + 'static>(
                &mut self,
                fut: F,
                scheduling_plan: impl SchedulingStrategy,
            ) {
                self.spawn(fut);
                self.run(scheduling_plan);
            }
        }

        /// Result of spawning a task.
        ///
        /// If you `.await` a JoinHandle, this will wait for the spawned task to complete.
        pub struct JoinHandle {
            index: usize,
        }

        impl Future for JoinHandle {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
                if unsafe { EXECUTOR.tasks[self.index].is_some() } {
                    std::task::Poll::Pending
                } else {
                    cx.waker().wake_by_ref(); // For completeness. But Kani's scheduler currently ignores wakers.
                    std::task::Poll::Ready(())
                }
            }
        }

        #[inline] // to work around linking issue
        pub fn spawn<F: Future<Output = ()> + Sync + 'static>(fut: F) -> JoinHandle {
            unsafe { EXECUTOR.spawn(fut) }
        }

        /// Polls the given future and the tasks it may spawn until all of them complete
        ///
        /// Contrary to block_on, this allows `spawn`ing other futures
        #[inline] // to work around linking issue
        pub fn spawnable_block_on<F: Future<Output = ()> + Sync + 'static>(
            fut: F,
            scheduling_plan: impl SchedulingStrategy,
        ) {
            unsafe {
                EXECUTOR.block_on(fut, scheduling_plan);
            }
        }

        /// Suspends execution of the current future, to allow the scheduler to poll another future
        ///
        /// Specifically, it returns a future that
        #[inline] // to work around linking issue
        pub fn yield_now() -> impl Future<Output = ()> {
            struct YieldNow {
                yielded: bool,
            }

            impl Future for YieldNow {
                type Output = ();

                fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
                    if self.yielded {
                        cx.waker().wake_by_ref(); // For completeness. But Kani's scheduler currently ignores wakers.
                        std::task::Poll::Ready(())
                    } else {
                        self.yielded = true;
                        std::task::Poll::Pending
                    }
                }
            }

            YieldNow { yielded: false }
        }
    }
}
