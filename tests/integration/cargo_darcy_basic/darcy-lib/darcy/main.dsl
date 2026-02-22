(require [math.add :as add])
(require [util.scale :as scale])

(defn calc [x:i64]
  (scale/scale (add/add3 x 2 3) 10))

(defn main []
  (calc 4))
