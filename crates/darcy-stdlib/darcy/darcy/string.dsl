(require [darcy.rust :refer [defextern]])

(defextern len [s:string] usize "darcy_stdlib::rt::string_len")
(defextern is-empty [s:string] bool "darcy_stdlib::rt::string_is_empty")
(defextern trim [s:string] string "darcy_stdlib::rt::string_trim")
(defextern split [s:string sep:string] vec<string> "darcy_stdlib::rt::string_split")
(defextern join [items:vec<string> sep:string] string "darcy_stdlib::rt::string_join")
