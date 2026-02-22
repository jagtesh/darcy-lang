(require [darcy.core])
(require [darcy.math])
(require [darcy.vec])
(require [darcy.tensor :as t])

(defrecord linear-layer
  (weights vec<vec<f64>>)
  (bias vec<f64>))

(defrecord train-state
  (layer linear-layer)
  (loss f64))

(defn sigmoid [x:f64]
  (/ 1.0 (+ 1.0 (darcy.math/exp (- 0.0 x)))))

(defn relu [x:f64]
  (if (darcy.op/gt x 0.0) x 0.0))

(defn softmax [v:vec<f64>]
  (let [exps (darcy.vec/map (fn [x:f64] (darcy.math/exp x)) v)
        total (t/vec-sum (darcy.core/clone exps))]
    (/ exps total)))


(defn linear-init [inputs:usize outputs:usize]
  (linear-layer
    (t/zeros2 outputs inputs)
    (t/zeros outputs)))

(defn linear-forward [layer:linear-layer x:vec<f64>]
  (let [weights (.weights (darcy.core/clone layer))
        bias (.bias (darcy.core/clone layer))]
    (t/vec-add (t/mat-vec weights x) bias)))

(defn predict [layer:linear-layer x:vec<f64>]
  (softmax (linear-forward layer x)))

(defn mse-loss [pred:vec<f64> target:vec<f64>]
  (let [diff (t/vec-sub pred target)]
    (* 0.5 (t/vec-sum (t/vec-mul (darcy.core/clone diff) diff)))))

(defn linear-backward [layer:linear-layer x:vec<f64> target:vec<f64> lr:f64]
  (let [pred (linear-forward (darcy.core/clone layer) (darcy.core/clone x))
        diff (t/vec-sub pred target)
        grad-w (t/mat-outer (darcy.core/clone diff) x)
        grad-b diff]
    (linear-layer
      (t/mat-sub (.weights (darcy.core/clone layer)) (t/mat-scale grad-w lr))
      (t/vec-sub (.bias (darcy.core/clone layer)) (t/vec-scale grad-b lr)))))

(defn train-step [state:train-state sample-x:vec<f64> sample-y:vec<f64> lr:f64]
  (let [pred (linear-forward (darcy.core/clone state.layer) (darcy.core/clone sample-x))
        loss (mse-loss pred (darcy.core/clone sample-y))
        layer2 (linear-backward (darcy.core/clone state.layer) sample-x sample-y lr)]
    (train-state layer2 (+ state.loss loss))))

(defn train-epoch [layer:linear-layer xs:vec<vec<f64>> ys:vec<vec<f64>> lr:f64]
  (let [samples (darcy.vec/map2 (fn [x:vec<f64> y:vec<f64>] [x y]) xs ys)
        init (train-state layer 0.0)
        out (darcy.vec/fold
              (fn [state:train-state pair:vec<vec<f64>>]
                (train-step state
                  (darcy.vec/get (darcy.core/clone pair) 0)
                  (darcy.vec/get pair 1)
                  lr))
              init
              samples)]
    out))
