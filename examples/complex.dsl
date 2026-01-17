(defstruct Order
  (id i32)
  (qty i32)
  (price f64))

(defn total-prices [os:Vec<Order>]
  (* os.price 2.0))

(defn main []
  (print (total-prices [(Order 1 2 3.5) (Order 2 4 1.25) (Order 3 1 9.0)])))
