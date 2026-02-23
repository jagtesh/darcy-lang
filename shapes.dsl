(defenum shape
  (triangle [b:f64 h:f64])
  (circle [r:f64])
  (square [s:f64])
  (rect [h:f64 w:f64]))


(defn area [s:shape]
  (let [pi 3.141592653589793]
    (case s
      (triangle (b b) (h h) (* b h))
      (circle (r r) (* pi (* r r)))
      (square (s s) (* s s))
      (rect (h h) (w w) (* h w)))))

(def blah {:a 1 :b 2 :c 3})

(defn main []
  (println "Area of triangle with base 5 and height 10: {}"
           (area (triangle 5.0 10.0))))
