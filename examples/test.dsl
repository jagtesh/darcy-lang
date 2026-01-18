(use core.fmt :only (dbg))

(defn main []
  (let [v (vec<i32> 1 2 3)]
    (for x v
      (dbg x))))
