```sh
~ ❯ wrk -t1 -c100 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  1 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   802.63us  363.13us  10.73ms   87.74%
    Req/Sec    76.52k     8.37k   90.86k    74.00%
  4569870 requests in 1.00m, 566.56MB read
Requests/sec:  76120.72
Transfer/sec:      9.44MB
~ ❯ wrk -t1 -c100 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  1 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     0.85ms    1.85ms  97.59ms   98.91%
    Req/Sec    74.53k     9.50k   91.46k    72.00%
  4450175 requests in 1.00m, 547.48MB read
Requests/sec:  74072.35
Transfer/sec:      9.11MB
~ ❯ wrk -t1 -c100 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  1 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   797.58us  377.37us  19.18ms   91.87%
    Req/Sec    73.05k     8.68k   90.27k    71.83%
  4360419 requests in 1.00m, 540.59MB read
Requests/sec:  72647.69
Transfer/sec:      9.01MB

~ ❯ wrk -t2 -c100 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  2 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.07ms    1.28ms  50.40ms   94.30%
    Req/Sec    43.20k    10.21k   77.40k    69.62%
  5161672 requests in 1.00m, 639.93MB read
Requests/sec:  85918.56
Transfer/sec:     10.65MB
~ ❯ wrk -t2 -c100 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  2 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.44ms    3.22ms  97.36ms   94.44%
    Req/Sec    46.02k    14.82k   84.28k    66.30%
  5486808 requests in 1.00m, 675.01MB read
Requests/sec:  91326.66
Transfer/sec:     11.24MB
~ ❯ wrk -t2 -c100 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  2 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     0.99ms    3.57ms 161.01ms   99.15%
    Req/Sec    50.18k     7.64k   83.37k    81.55%
  5988543 requests in 1.00m, 742.45MB read
Requests/sec:  99660.28
Transfer/sec:     12.36MB

~ ❯ wrk -t4 -c500 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  4 threads and 500 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.73ms    2.95ms 111.31ms   91.27%
    Req/Sec    21.38k     6.90k   50.17k    67.84%
  5100968 requests in 1.00m, 632.41MB read
  Socket errors: connect 253, read 40, write 0, timeout 0
Requests/sec:  84948.95
Transfer/sec:     10.53MB
~ ❯ wrk -t4 -c500 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  4 threads and 500 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     5.20ms   11.95ms 192.69ms   91.28%
    Req/Sec    24.14k    10.80k   65.80k    68.55%
  5763960 requests in 1.00m, 709.11MB read
  Socket errors: connect 253, read 95, write 0, timeout 0
Requests/sec:  95915.59
Transfer/sec:     11.80MB
~ ❯ wrk -t4 -c500 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  4 threads and 500 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     4.59ms   11.64ms 251.65ms   93.06%
    Req/Sec    25.78k     8.59k   58.77k    69.01%
  6149015 requests in 1.00m, 762.34MB read
  Socket errors: connect 253, read 66, write 0, timeout 0
Requests/sec: 102367.77
Transfer/sec:     12.69MB

~ ❯ wrk -t2 -c1000 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  2 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.28ms    1.54ms  93.10ms   82.09%
    Req/Sec    42.95k     8.01k   66.85k    71.90%
  5126898 requests in 1.00m, 635.62MB read
  Socket errors: connect 751, read 28, write 0, timeout 0
Requests/sec:  85346.03
Transfer/sec:     10.58MB
~ ❯ wrk -t2 -c1000 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  2 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.01ms    1.93ms  76.62ms   89.14%
    Req/Sec    47.06k     8.11k   86.34k    72.12%
  5620597 requests in 1.00m, 691.47MB read
  Socket errors: connect 751, read 75, write 0, timeout 0
Requests/sec:  93530.45
Transfer/sec:     11.51MB
~ ❯ wrk -t2 -c1000 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  2 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.91ms    1.77ms 126.99ms   92.01%
    Req/Sec    49.31k     7.43k   80.33k    77.82%
  5886996 requests in 1.00m, 729.86MB read
  Socket errors: connect 751, read 0, write 0, timeout 0
Requests/sec:  97985.39
Transfer/sec:     12.15MB

~ ❯ wrk -t4 -c1000 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  4 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.79ms    3.63ms 212.98ms   93.38%
    Req/Sec    20.74k     7.07k   54.07k    71.72%
  4943103 requests in 1.00m, 612.83MB read
  Socket errors: connect 753, read 33, write 7, timeout 0
Requests/sec:  82274.50
Transfer/sec:     10.20MB
~ ❯ wrk -t4 -c1000 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  4 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     3.88ms   11.05ms 313.05ms   93.40%
    Req/Sec    24.07k    10.99k   62.69k    70.50%
  5737235 requests in 1.00m, 705.82MB read
  Socket errors: connect 753, read 95, write 0, timeout 0
Requests/sec:  95495.52
Transfer/sec:     11.75MB
~ ❯ wrk -t4 -c1000 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  4 threads and 1000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     3.32ms    9.44ms 201.49ms   94.51%
    Req/Sec    26.85k     9.25k   66.83k    70.79%
  6413429 requests in 1.00m, 795.12MB read
  Socket errors: connect 753, read 86, write 0, timeout 0
Requests/sec: 106712.48
Transfer/sec:     13.23MB

~ ❯ wrk -t4 -c2000 -d60s http://localhost:8080
Running 1m test @ http://localhost:8080
  4 threads and 2000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.68ms    2.62ms 135.28ms   90.28%
    Req/Sec    21.06k     7.06k   51.80k    69.98%
  5020996 requests in 1.00m, 622.49MB read
  Socket errors: connect 1753, read 63, write 0, timeout 0
Requests/sec:  83603.25
Transfer/sec:     10.36MB
~ ❯ wrk -t4 -c2000 -d60s http://localhost:8081
Running 1m test @ http://localhost:8081
  4 threads and 2000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.93ms    8.64ms 228.52ms   95.16%
    Req/Sec    23.58k    10.51k   61.80k    70.46%
  5628894 requests in 1.00m, 692.49MB read
  Socket errors: connect 1753, read 97, write 0, timeout 0
Requests/sec:  93654.80
Transfer/sec:     11.52MB
~ ❯ wrk -t4 -c2000 -d60s http://localhost:8082
Running 1m test @ http://localhost:8082
  4 threads and 2000 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.88ms    8.72ms 204.46ms   96.03%
    Req/Sec    24.72k     9.14k   63.78k    69.96%
  5895430 requests in 1.00m, 730.90MB read
  Socket errors: connect 1753, read 36, write 0, timeout 0
Requests/sec:  98091.72
Transfer/sec:     12.16MB

PID    COMMAND      %CPU  TIME     #TH  #WQ  #POR MEM
93470  native-axum  0.0   10:54.36 5    0    15   12M
93423  native-actix 0.0   10:28.28 6    0    17   6904K
93378  native-ntex  0.0   10:08.08 8    0    20   5252K
```