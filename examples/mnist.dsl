(require [darcy.fmt :refer [println]])
(require [darcy.mnist :as mnist])
(require [darcy.nn :as nn])
(require [darcy.tensor :as t])
(require [darcy.vec :as v])

(defrecord sample
  (x vec<f64>)
  (y vec<f64>))

(defrecord acc
  (correct f64)
  (total f64))

(defn argmax-score [v:vec<f64>]
  (v/fold (fn [acc:f64 x:f64] (if (darcy.math/gt x acc) x acc)) -1.0 v))

(defn is-correct [layer:nn/linear-layer s:sample]
  (let [pred (nn/predict layer s.x)
        maxv (argmax-score (darcy.core/clone pred))
        truev (t/vec-dot pred s.y)]
    (if (darcy.math/eq truev maxv) 1.0 0.0)))

(defn accuracy [layer:nn/linear-layer xs:vec<vec<f64>> ys:vec<vec<f64>>]
  (let [pairs (v/map2 (fn [x y] (sample x y)) xs ys)
        stats (v/fold
          (fn [a:acc s:sample]
            (acc (+ a.correct (is-correct (darcy.core/clone layer) s)) (+ a.total 1.0)))
          (acc 0.0 0.0)
          pairs)]
    (/ stats.correct stats.total)))

(defn train-n [state:nn/train-state xs:vec<vec<f64>> ys:vec<vec<f64>> lr:f64 epochs:i32]
  (if (darcy.math/eq epochs 0)
    state
    (train-n
      (nn/train-epoch state.layer (darcy.core/clone xs) (darcy.core/clone ys) lr)
      xs
      ys
      lr
      (- epochs 1))))

(defn main []
  (let [data (mnist/load-edn-gz "clojure-neural-networks-from-scratch/resources/mnist/training_data.edn.gz")
        xs (v/take data.images 200)
        ys (v/take data.labels 200)
        layer (nn/linear-init 784 10)
        state0 (nn/train-epoch layer (darcy.core/clone xs) (darcy.core/clone ys) 0.01)
        state (train-n state0 (darcy.core/clone xs) (darcy.core/clone ys) 0.01 2)
        acc (accuracy state.layer xs ys)]
    (do
      (println "loss={}" state.loss)
      (println "acc={}" acc))))
