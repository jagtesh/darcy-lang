(defrecord order
  (id i32)
  (qty i32)
  (price f64))


(defn total-prices [os:vec<order>]
  (* os.price 2.0))

(defn main []
  (darcy.io/dbg (total-prices [(order 1 2 3.5) (order 2 4 1.25) (order 3 1 9.0)])))
