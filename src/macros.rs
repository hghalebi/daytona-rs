/// Runs one checked command. Returns the same builder as the receiver's run method.
///
/// Optional named settings: ttl (client only) and execution_timeout. Each accepts
/// a typed value or an integer literal with min/sec respectively. Options may be
/// in either order and may appear once. Arguments evaluate once in source order.
/// SDK validation errors are returned when awaited. No hidden client is created.
#[macro_export]
macro_rules! run {
    ($receiver:expr, $script:expr $(, $($options:tt)*)?) => {
        $crate::__run_options!(($receiver).run($script); [no no]; $($($options)*)?)
    };
}
/// Sets a positive lifetime in minutes. Returns Result<SandboxTtl, ValidationError>.
#[macro_export]
macro_rules! ttl {
    ($value:literal min $(,)?) => {
        $crate::SandboxTtl::minutes($value)
    };
    ($($invalid:tt)*) => {
        compile_error!("use ttl!(5 min); dynamic values use SandboxTtl::minutes(value)")
    };
}
/// Sets a positive command limit in seconds. Returns Result<ExecutionTimeout, ValidationError>.
#[macro_export]
macro_rules! execution_timeout {
    ($value:literal sec $(,)?) => {
        $crate::ExecutionTimeout::seconds($value)
    };
    ($($invalid:tt)*) => {
        compile_error!(
            "use execution_timeout!(30 sec); dynamic values use ExecutionTimeout::seconds(value)"
        )
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __run_options {
    ($task:expr; [$ttl:ident $timeout:ident];) => { $task };
    ($task:expr; [yes $timeout:ident]; ttl: $($rest:tt)*) => { compile_error!("duplicate ttl option") };
    ($task:expr; [$ttl:ident yes]; execution_timeout: $($rest:tt)*) => { compile_error!("duplicate execution_timeout option") };
    ($task:expr; [no $timeout:ident]; ttl: $value:literal min $(, $($rest:tt)*)?) => {
        $crate::__run_options!($task.__ttl($crate::ttl!($value min)); [yes $timeout]; $($($rest)*)?)
    };
    ($task:expr; [no $timeout:ident]; ttl: $value:expr $(, $($rest:tt)*)?) => {
        $crate::__run_options!($task.__ttl(::core::result::Result::Ok($value)); [yes $timeout]; $($($rest)*)?)
    };
    ($task:expr; [$ttl:ident no]; execution_timeout: $value:literal sec $(, $($rest:tt)*)?) => {
        $crate::__run_options!($task.__execution_timeout($crate::execution_timeout!($value sec)); [$ttl yes]; $($($rest)*)?)
    };
    ($task:expr; [$ttl:ident no]; execution_timeout: $value:expr $(, $($rest:tt)*)?) => {
        $crate::__run_options!($task.__execution_timeout(::core::result::Result::Ok($value)); [$ttl yes]; $($($rest)*)?)
    };
    ($task:expr; [$ttl:ident $timeout:ident]; $($invalid:tt)+) => {
        compile_error!("use ttl: 5 min or execution_timeout: 30 sec, or a value of the corresponding domain type")
    };
}
/// Lends one temporary sandbox to an async callback, then confirms deletion.
///
/// Returns a ScopeTask with a reference available before await. The work option
/// is required; ttl is optional and may precede or follow work. Work is captured
/// once and its body is unchanged. Cancellation does not perform async cleanup.
#[macro_export]
macro_rules! scope {
    ($client:expr, work: $work:expr $(,)?) => { ($client).scope($work) };
    ($client:expr, ttl: $ttl:literal min, work: $work:expr $(,)?) => {
        $crate::__scope_before!($client, $crate::ttl!($ttl min), $work)
    };
    ($client:expr, ttl: $ttl:expr, work: $work:expr $(,)?) => {
        $crate::__scope_before!($client, ::core::result::Result::Ok($ttl), $work)
    };
    ($client:expr, work: $work:expr, ttl: $ttl:literal min $(,)?) => {
        ($client).scope($work).__ttl($crate::ttl!($ttl min))
    };
    ($client:expr, work: $work:expr, ttl: $ttl:expr $(,)?) => {
        ($client).scope($work).__ttl(::core::result::Result::Ok($ttl))
    };
    ($($invalid:tt)*) => {
        compile_error!("scope! requires one client, one work: async callback, and optionally one ttl: 5 min or typed TTL")
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __scope_before {
    ($client:expr, $ttl:expr, $work:expr) => {{
        let __daytona_client = $client;
        let __daytona_ttl = $ttl;
        __daytona_client.scope($work).__ttl(__daytona_ttl)
    }};
}
