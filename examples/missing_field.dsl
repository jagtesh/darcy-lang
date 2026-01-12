(defstruct Order
  (qty u32)
  (price f64))

(defn total [o:Order]
  (* o.qty o.cost))
