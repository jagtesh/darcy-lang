(require [darcy.io :refer [dbg]])

(defn main []
  (let [v (vec<i32> 1 2 3)]
    (for x v
      (darcy.io/dbg x))))
