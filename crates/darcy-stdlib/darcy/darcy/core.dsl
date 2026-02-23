(require [darcy.rust :refer [defextern]])
(require [darcy.math])
(require [darcy.op])

(defextern clone [v:t0] t0 "darcy_stdlib::rt::core_clone")

;; helper macro for adding simple function aliases without needing defin
(defmacro defn-alias [name target]
  `(defmacro ~name [& args]
    `(~target ~@args))

(defin add [a b] (darcy.op/add a b))
(defin sub [a b] (darcy.op/sub a b))
(defin mul [a b] (darcy.op/mul a b))
(defin div [a b] (darcy.op/div a b))
(defin mod [a b] (darcy.op/mod a b))

(defin eq [a b] (darcy.op/eq a b))
(defin lt [a b] (darcy.op/lt a b))
(defin lte [a b] (darcy.op/lte a b))
(defin gt [a b] (darcy.op/gt a b))
(defin gte [a b] (darcy.op/gte a b))
(defin bit-and [a b] (darcy.op/bit-and a b))
(defin bit-or [a b] (darcy.op/bit-or a b))

(defin exp [x] (darcy.math/exp x))
(defin abs [x] (darcy.math/abs x))
(defin min [a b] (darcy.math/min a b))
(defin max [a b] (darcy.math/max a b))
(defin clamp [x lo hi] (darcy.math/clamp x lo hi))

;; built-in io/format helpers
(defn-alias println darcy.fmt/println)
(defn-alias print darcy.fmt/print)
(defn-alias dbg darcy.io/dbg)
(defn-alias format darcy.fmt/format)
