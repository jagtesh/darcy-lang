(require [darcy.core])

(defrecord rand-state
  (seed i64))

(defrecord rand-i64-pair
  (state rand-state)
  (value i64))

(defrecord rand-f64-pair
  (state rand-state)
  (value f64))

(defn seed [seed:i64]
  (rand-state seed))

(defn mod-i64 [x:i64 m:i64]
  (- x (* (/ x m) m)))

(defn next-state [state:rand-state]
  (let [a 48271
        m 2147483647
        prod (* state.seed a)
        next (mod-i64 prod m)]
    (rand-state next)))

(defn rand-i64 [state:rand-state]
  (let [next (next-state state)
        state2 (darcy.core/clone next)]
    (rand-i64-pair state2 next.seed)))

(defn rand-f64 [state:rand-state]
  (let [pair (rand-i64 state)
        s pair.state
        v pair.value
        out (/ v 2147483647.0)]
    (rand-f64-pair s out)))

(defn rand-normal [state:rand-state]
  (let [p1 (rand-f64 state)
        s1 p1.state
        u1 p1.value
        p2 (rand-f64 s1)
        s2 p2.state
        u2 p2.value
        p3 (rand-f64 s2)
        s3 p3.state
        u3 p3.value
        p4 (rand-f64 s3)
        s4 p4.state
        u4 p4.value
        p5 (rand-f64 s4)
        s5 p5.state
        u5 p5.value
        p6 (rand-f64 s5)
        s6 p6.state
        u6 p6.value
        sum1 (+ u1 u2)
        sum2 (+ sum1 u3)
        sum3 (+ sum2 u4)
        sum4 (+ sum3 u5)
        sum5 (+ sum4 u6)
        z (- sum5 3.0)]
    (rand-f64-pair s6 z)))

(defn rand-range [state:rand-state lo:i64 hi:i64]
  (let [pair (rand-i64 state)
        s pair.state
        v pair.value
        span (- hi lo)
        off (mod-i64 v span)
        out (+ lo off)]
    (rand-i64-pair s out)))
