(require [darcy.rust :refer [defextern]])

(defextern exp [x:f64] f64 "darcy_stdlib::rt::math_exp")
(defextern abs [x:t0] t0 "darcy_stdlib::rt::math_abs")
(defextern min [a:t0 b:t0] t0 "darcy_stdlib::rt::math_min")
(defextern max [a:t0 b:t0] t0 "darcy_stdlib::rt::math_max")
(defextern clamp [x:t0 lo:t0 hi:t0] t0 "darcy_stdlib::rt::math_clamp")
