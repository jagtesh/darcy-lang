(require [darcy.math :as math])
(require [darcy.vec])
(require [darcy.core])

(defrecord pixel (px i32) (py i32) (pr i32) (pg i32) (pb i32))

(defrecord complex (re f64) (im f64))

(defenum fractal-type (mandelbrot) (julia) (sierpinski))

(defrecord fractal-config 
  (width i32) 
  (height i32) 
  (fractal-type fractal-type)
  (max-iter i32)
  (min-re f64)
  (max-re f64)
  (min-im f64)
  (max-im f64)
  (julia-re f64)
  (julia-im f64))

(defin complex-abs-sq [c:complex]
  (+ (* c.re c.re) (* c.im c.im)))

(defin complex-add [c1:complex c2:complex]
  (complex (+ c1.re c2.re) (+ c1.im c2.im)))

(defin complex-mul [c1:complex c2:complex]
  (complex (- (* c1.re c2.re) (* c1.im c2.im)) 
          (+ (* c1.re c2.im) (* c1.im c2.re))))

(defin mod-custom [x:i64 m:i64]
  (- x (* m (cast (/ (cast x f64) (cast m f64)) i64))))

(defin map-color [iterations:i32 max:i32]
  (if (darcy.op/eq iterations max)
    0
    (let [t (/ (cast iterations f64) (cast max f64))]
      (cast (* 255.0 (* t (- 1.0 t))) i32))))

(defin mandelbrot-at [x:i32 y:i32 config:fractal-config]
  (let [re (+ config.min-re (* (cast x f64) (/ (- config.max-re config.min-re) (cast config.width f64))))
        im (+ config.min-im (* (cast y f64) (/ (- config.max-im config.min-im) (cast config.height f64))))
        c (complex re im)]
    (let [z (complex 0.0 0.0) i 0]
      (while (and (darcy.op/lt i config.max-iter) (darcy.op/lt (complex-abs-sq z) 4.0))
        (do
          (let! z (complex-add (complex-mul z z) c))
          (let! i (+ i 1))))
      i)))

(defin julia-at [x:i32 y:i32 config:fractal-config]
  (let [re (+ config.min-re (* (cast x f64) (/ (- config.max-re config.min-re) (cast config.width f64))))
        im (+ config.min-im (* (cast y f64) (/ (- config.max-im config.min-im) (cast config.height f64))))
        z (complex re im)
        c (complex config.julia-re config.julia-im)]
    (let [i 0]
      (while (and (darcy.op/lt i config.max-iter) (darcy.op/lt (complex-abs-sq z) 4.0))
        (do
          (let! z (complex-add (complex-mul z z) c))
          (let! i (+ i 1))))
      i)))

(defin sierpinski-at [x:i32 y:i32 config:fractal-config]
  (let [dx (math/abs (- x (cast (/ config.width 2) i32)))
        dy (math/abs (- y (cast (/ config.height 2) i32)))
        level config.max-iter]
    (let [i 0]
      (while (darcy.op/lt i level)
        (do
          (let! dx (cast (/ (cast dx f64) 2.0) i32))
          (let! dy (cast (/ (cast dy f64) 2.0) i32))
          (let! i (+ i 1))))
      (if (and (darcy.op/eq (mod-custom (cast dx i64) 2) 1) (darcy.op/eq (mod-custom (cast dy i64) 2) 1))
        0
        level))))

(defin generate-pixel [config:fractal-config x:i32 y:i32]
  (let [iterations (case config.fractal-type
                      (mandelbrot (mandelbrot-at x y config))
                      (julia (julia-at x y config))
                      (sierpinski (sierpinski-at x y config)))
        red (map-color iterations config.max-iter)
        green (map-color iterations config.max-iter)
        blue (map-color iterations config.max-iter)]
    (pixel x y red green blue)))

(defn generate-fractal [config:fractal-config]
  (let [pixels (darcy.vec/new)]
    (let [y 0]
      (while (darcy.op/lt y config.height)
        (do
          (let [x 0]
            (while (darcy.op/lt x config.width)
              (do
                (darcy.vec/push pixels (generate-pixel config x y))
                (let! x (+ x 1)))))
          (let! y (+ y 1)))))
    pixels))

(export (defn generate-mandelbrot [width:i32 height:i32 max-iter:i32]
  (let [config (fractal-config 
                width 
                height 
                (mandelbrot) 
                max-iter
                -2.5
                1.0
                -1.0
                1.0
                0.0
                0.0)]
    (generate-fractal config))))

(export (defn generate-julia [width:i32 height:i32 max-iter:i32 cre:f64 cim:f64]
  (let [config (fractal-config 
                width 
                height 
                (julia) 
                max-iter
                -2.0
                2.0
                -2.0
                2.0
                cre
                cim)]
    (generate-fractal config))))

(export (defn generate-sierpinski [width:i32 height:i32 max-level:i32]
  (let [config (fractal-config 
                width 
                height 
                (sierpinski) 
                max-level
                0.0
                0.0
                0.0
                0.0
                0.0
                0.0)]
    (generate-fractal config))))

(defn main []
  (darcy.io/dbg "Generated Mandelbrot fractal")
  (darcy.io/dbg "Generated Julia fractal")
  (darcy.io/dbg "Generated Sierpinski triangle"))
