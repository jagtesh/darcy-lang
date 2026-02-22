(require [darcy.rust :refer [defextern]])

(defextern some [val:t0] option<t0> "darcy_stdlib::rt::option_some")
(defextern none [] option<t0> "darcy_stdlib::rt::option_none")
(defextern is-some [val:option<t0>] bool "darcy_stdlib::rt::option_is_some")
(defextern is-none [val:option<t0>] bool "darcy_stdlib::rt::option_is_none")
(defextern unwrap [val:option<t0>] t0 "darcy_stdlib::rt::option_unwrap")
(defextern unwrap-or [val:option<t0> fallback:t0] t0 "darcy_stdlib::rt::option_unwrap_or")
