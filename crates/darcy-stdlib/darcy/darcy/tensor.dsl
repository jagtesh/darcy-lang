(require [darcy.core])
(require [darcy.vec])


(defn zeros [n:usize]
  (darcy.vec/repeat 0.0 n))

(defn ones [n:usize]
  (darcy.vec/repeat 1.0 n))

(defn zeros2 [rows:usize cols:usize]
  (darcy.vec/repeat (zeros cols) rows))

(defn ones2 [rows:usize cols:usize]
  (darcy.vec/repeat (ones cols) rows))


(defn vec-add [a b]
  (darcy.vec/map2 (fn [x y] (+ x y)) a b))

(defn vec-sub [a b]
  (darcy.vec/map2 (fn [x y] (- x y)) a b))

(defn vec-mul [a b]
  (darcy.vec/map2 (fn [x y] (* x y)) a b))

(defn vec-scale [v s]
  (darcy.vec/map (fn [x] (* x s)) v))

(defn vec-sum [v]
  (darcy.vec/fold (fn [acc x] (+ acc x)) 0.0 v))

(defn vec-dot [a:vec<f64> b:vec<f64>]
  (darcy.vec/fold (fn [acc x] (+ acc x)) 0.0
    (darcy.vec/map2 (fn [x y] (* x y)) a b)))

(defn mat-vec [m:vec<vec<f64>> v:vec<f64>]
  (darcy.vec/map (fn [row] (vec-dot row (darcy.core/clone v))) m))

(defn mat-add [a:vec<vec<f64>> b:vec<vec<f64>>]
  (darcy.vec/map2 (fn [ra rb] (vec-add ra rb)) a b))

(defn mat-sub [a:vec<vec<f64>> b:vec<vec<f64>>]
  (darcy.vec/map2 (fn [ra rb] (vec-sub ra rb)) a b))

(defn mat-outer [a:vec<f64> b:vec<f64>]
  (darcy.vec/map (fn [x] (vec-scale (darcy.core/clone b) x)) a))

(defn mat-scale [m:vec<vec<f64>> s:f64]
  (darcy.vec/map (fn [row] (vec-scale row s)) m))
