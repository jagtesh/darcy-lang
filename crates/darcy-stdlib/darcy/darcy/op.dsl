(defn add [a b] (+ a b))
(defn sub [a b] (- a b))
(defn mul [a b] (* a b))
(defn div [a b] (/ a b))
(defn mod [a b] (mod a b))

(defn eq [a b] (= a b))
(defn lt [a b] (< a b))
(defn lte [a b] (<= a b))
(defn gt [a b] (> a b))
(defn gte [a b] (>= a b))

(defn bit-and [a:i64 b:i64] (& a b))
(defn bit-or [a:i64 b:i64] (| a b))
