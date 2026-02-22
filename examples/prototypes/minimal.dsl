(require [darcy.vec :as v])
(require [darcy.io :as io])

(defn main []
  (let [prices (vec<f64>)]
    (do
      (v/push prices 100.0)
      (let [l (v/len prices)]
        (io/dbg l)))))