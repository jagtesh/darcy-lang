(require [darcy.rust :refer [defextern]])

(defextern format [val:t0] string "darcy_stdlib::rt::fmt_format")
(defextern pretty [val:t0] string "darcy_stdlib::rt::fmt_pretty")
(defextern print [s:string] unit "darcy_stdlib::rt::fmt_print")
(defextern println [s:string] unit "darcy_stdlib::rt::fmt_println")
