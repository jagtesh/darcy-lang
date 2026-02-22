(require [darcy.rust :refer [defextern defextern-record]])

(defextern-record mnist-data "darcy_runtime::mnist::MnistData"
  [(images vec<vec<f64>>) (labels vec<vec<f64>>)])

(defextern load-edn-gz [path:string] mnist-data "darcy_runtime::mnist::load_edn_gz")
