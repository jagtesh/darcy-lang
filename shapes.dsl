;; (defenum shape
;;   (triangle [b:f64 h:f64])
;;   (circle [r:f64])
;;   (square [s:f64])
;;   (rect [h:f64 w:f64]))

(defrecord triangle
  [b:f64 h:f64])

(defrecord circle [r:f64])

(defrecord square [s:f64])

(defrecord rect [h:f64 w:f64])

(defn area [shape]
  (case shape
    (triangle (* shape.b shape.h))))

(defn main []
  (darcy.fmt/println "Area of triangle with base 5 and height 10: {}"
                     (area (triangle 5.0 10.0))))
