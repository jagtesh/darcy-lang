(require [darcy.rust :refer [defextern]])

(defextern ok [val:t0] result<t0,t1> "darcy_stdlib::rt::result_ok")
(defextern err [err:t1] result<t0,t1> "darcy_stdlib::rt::result_err")
(defextern is-ok [val:result<t0,t1>] bool "darcy_stdlib::rt::result_is_ok")
(defextern is-err [val:result<t0,t1>] bool "darcy_stdlib::rt::result_is_err")
(defextern unwrap [val:result<t0,t1>] t0 "darcy_stdlib::rt::result_unwrap")
(defextern unwrap-or [val:result<t0,t1> fallback:t0] t0 "darcy_stdlib::rt::result_unwrap_or")
