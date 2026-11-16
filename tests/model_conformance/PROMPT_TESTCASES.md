# PROMPT_TESTCASES.md: Turing-Complete 10-Category End-User Prompt Specification

## CAT-01: Core Arithmetic, Numeric Formulas & Unit Conversions

### TC-01-01: Order Subtotal Plus Tax
- **Category**: `CAT-01`
- **Prompt**: `Hey, write a quick function calc_order_total that adds subtotal and tax for a checkout receipt.`
- **Signature Hint**: `f calc_order_total (subtotal tax) [`
- **Test Call (pIR)**: `p (calc_order_total 120 18)`
- **Test Call (.psudo)**: `print calc_order_total(120, 18)`
- **Expected Stdout**: `138`
```pir
f calc_order_total (subtotal tax) [
  v total (+ subtotal tax)
  r total
]
```

### TC-01-02: Celsius to Fahrenheit
- **Category**: `CAT-01`
- **Prompt**: `Convert Celsius to Fahrenheit in a function celsius_to_fahrenheit(c_deg) using c_deg * 9 / 5 + 32.`
- **Signature Hint**: `f celsius_to_fahrenheit (c_deg) [`
- **Test Call (pIR)**: `p (celsius_to_fahrenheit 100)`
- **Test Call (.psudo)**: `print celsius_to_fahrenheit(100)`
- **Expected Stdout**: `212`
```pir
f celsius_to_fahrenheit (c_deg) [
  v scaled (/ (* c_deg 9) 5)
  r (+ scaled 32)
]
```

### TC-01-03: Rectangular Plot Perimeter
- **Category**: `CAT-01`
- **Prompt**: `I need rect_perimeter(width, height) to calculate the perimeter around a rectangular plot.`
- **Signature Hint**: `f rect_perimeter (width height) [`
- **Test Call (pIR)**: `p (rect_perimeter 14 6)`
- **Test Call (.psudo)**: `print rect_perimeter(14, 6)`
- **Expected Stdout**: `40`
```pir
f rect_perimeter (width height) [
  r (* 2 (+ width height))
]
```

### TC-01-04: Triangle Area
- **Category**: `CAT-01`
- **Prompt**: `Write triangle_area(base_len, height) returning half of base times height.`
- **Signature Hint**: `f triangle_area (base_len height) [`
- **Test Call (pIR)**: `p (triangle_area 12 10)`
- **Test Call (.psudo)**: `print triangle_area(12, 10)`
- **Expected Stdout**: `60`
```pir
f triangle_area (base_len height) [
  r (/ (* base_len height) 2)
]
```

### TC-01-05: Simple Interest
- **Category**: `CAT-01`
- **Prompt**: `Create simple_interest(principal, rate_pct, years) for principal * rate_pct * years / 100.`
- **Signature Hint**: `f simple_interest (principal rate_pct years) [`
- **Test Call (pIR)**: `p (simple_interest 1000 5 3)`
- **Test Call (.psudo)**: `print simple_interest(1000, 5, 3)`
- **Expected Stdout**: `150`
```pir
f simple_interest (principal rate_pct years) [
  v numer (* (* principal rate_pct) years)
  r (/ numer 100)
]
```

### TC-01-06: Average of Three Scores
- **Category**: `CAT-01`
- **Prompt**: `How do I write avg_three(s1, s2, s3) to compute the mean of three exam scores?`
- **Signature Hint**: `f avg_three (s1 s2 s3) [`
- **Test Call (pIR)**: `p (avg_three 80 90 100)`
- **Test Call (.psudo)**: `print avg_three(80, 90, 100)`
- **Expected Stdout**: `90`
```pir
f avg_three (s1 s2 s3) [
  v sum_all (+ (+ s1 s2) s3)
  r (/ sum_all 3)
]
```

### TC-01-07: Travel Distance
- **Category**: `CAT-01`
- **Prompt**: `fun to compute travel_distance given speed and hours driven`
- **Signature Hint**: `f travel_distance (speed hours) [`
- **Test Call (pIR)**: `p (travel_distance 65 4)`
- **Test Call (.psudo)**: `print travel_distance(65, 4)`
- **Expected Stdout**: `260`
```pir
f travel_distance (speed hours) [
  r (* speed hours)
]
```

### TC-01-08: Percentage Discount
- **Category**: `CAT-01`
- **Prompt**: `Implement apply_pct_discount(price, pct) that subtracts pct percent off price.`
- **Signature Hint**: `f apply_pct_discount (price pct) [`
- **Test Call (pIR)**: `p (apply_pct_discount 200 15)`
- **Test Call (.psudo)**: `print apply_pct_discount(200, 15)`
- **Expected Stdout**: `170`
```pir
f apply_pct_discount (price pct) [
  v cut (/ (* price pct) 100)
  r (- price cut)
]
```

### TC-01-09: Split Bill With Tip
- **Category**: `CAT-01`
- **Prompt**: `Build a dinner helper split_bill_with_tip(bill, tip, people) returning each person's share.`
- **Signature Hint**: `f split_bill_with_tip (bill tip people) [`
- **Test Call (pIR)**: `p (split_bill_with_tip 150 30 6)`
- **Test Call (.psudo)**: `print split_bill_with_tip(150, 30, 6)`
- **Expected Stdout**: `30`
```pir
f split_bill_with_tip (bill tip people) [
  r (/ (+ bill tip) people)
]
```

### TC-01-10: Weighted Grade
- **Category**: `CAT-01`
- **Prompt**: `Write weighted_grade(exam, hw) where exam counts 70% and homework counts 30%.`
- **Signature Hint**: `f weighted_grade (exam hw) [`
- **Test Call (pIR)**: `p (weighted_grade 90 80)`
- **Test Call (.psudo)**: `print weighted_grade(90, 80)`
- **Expected Stdout**: `87`
```pir
f weighted_grade (exam hw) [
  r (/ (+ (* exam 70) (* hw 30)) 100)
]
```


## CAT-02: Conditional Branching, Multi-Way Dispatch & Boundary Guards

### TC-02-01: Absolute Difference
- **Category**: `CAT-02`
- **Prompt**: `Write abs_diff(pa, pb) that returns the positive distance between two numbers.`
- **Signature Hint**: `f abs_diff (pa pb) [`
- **Test Call (pIR)**: `p (abs_diff 12 29)`
- **Test Call (.psudo)**: `print abs_diff(12, 29)`
- **Expected Stdout**: `17`
```pir
f abs_diff (pa pb) [
  ? (> pa pb) [
    r (- pa pb)
  ] [
    r (- pb pa)
  ]
]
```

### TC-02-02: Clamp Value
- **Category**: `CAT-02`
- **Prompt**: `Implement clamp_val(val, low, high) so val never drops below low or exceeds high.`
- **Signature Hint**: `f clamp_val (val low high) [`
- **Test Call (pIR)**: `p (clamp_val 150 0 100)`
- **Test Call (.psudo)**: `print clamp_val(150, 0, 100)`
- **Expected Stdout**: `100`
```pir
f clamp_val (val low high) [
  ? (< val low) [
    r low
  ]
  ? (> val high) [
    r high
  ]
  r val
]
```

### TC-02-03: Max of Three
- **Category**: `CAT-02`
- **Prompt**: `Give me max_of_three(pa, pb, pc) returning the largest of three numbers.`
- **Signature Hint**: `f max_of_three (pa pb pc) [`
- **Test Call (pIR)**: `p (max_of_three 14 42 27)`
- **Test Call (.psudo)**: `print max_of_three(14, 42, 27)`
- **Expected Stdout**: `42`
```pir
f max_of_three (pa pb pc) [
  v best pa
  ? (> pb best) [
    = best pb
  ]
  ? (> pc best) [
    = best pc
  ]
  r best
]
```

### TC-02-04: Min of Three
- **Category**: `CAT-02`
- **Prompt**: `fun to find min_of_three(pa, pb, pc) returning the smallest argument`
- **Signature Hint**: `f min_of_three (pa pb pc) [`
- **Test Call (pIR)**: `p (min_of_three 14 7 27)`
- **Test Call (.psudo)**: `print min_of_three(14, 7, 27)`
- **Expected Stdout**: `7`
```pir
f min_of_three (pa pb pc) [
  v smallest pa
  ? (< pb smallest) [
    = smallest pb
  ]
  ? (< pc smallest) [
    = smallest pc
  ]
  r smallest
]
```

### TC-02-05: Signum Function
- **Category**: `CAT-02`
- **Prompt**: `Code signum_val(num_x) returning 1 for positive, -1 for negative, and 0 for zero.`
- **Signature Hint**: `f signum_val (num_x) [`
- **Test Call (pIR)**: `p (signum_val -45)`
- **Test Call (.psudo)**: `print signum_val(-45)`
- **Expected Stdout**: `-1`
```pir
f signum_val (num_x) [
  ? (> num_x 0) [
    r 1
  ]
  ? (< num_x 0) [
    r -1
  ]
  r 0
]
```

### TC-02-06: Tax Bracket Rate
- **Category**: `CAT-02`
- **Prompt**: `As payroll, write tax_bracket_rate(income): 0 under 20000, 15 under 80000, otherwise 28.`
- **Signature Hint**: `f tax_bracket_rate (income) [`
- **Test Call (pIR)**: `p (tax_bracket_rate 55000)`
- **Test Call (.psudo)**: `print tax_bracket_rate(55000)`
- **Expected Stdout**: `15`
```pir
f tax_bracket_rate (income) [
  ? (< income 20000) [
    r 0
  ]
  ? (< income 80000) [
    r 15
  ]
  r 28
]
```

### TC-02-07: Shipping Fee
- **Category**: `CAT-02`
- **Prompt**: `Write shipping_fee(order_amt, weight_kg): free (0) from 100+, 25 if weight_kg > 10, else 10.`
- **Signature Hint**: `f shipping_fee (order_amt weight_kg) [`
- **Test Call (pIR)**: `p (shipping_fee 60 12)`
- **Test Call (.psudo)**: `print shipping_fee(60, 12)`
- **Expected Stdout**: `25`
```pir
f shipping_fee (order_amt weight_kg) [
  ? (>= order_amt 100) [
    r 0
  ]
  ? (> weight_kg 10) [
    r 25
  ]
  r 10
]
```

### TC-02-08: HTTP Status Family
- **Category**: `CAT-02`
- **Prompt**: `Create http_status_family(code) mapping 200..299->2, 400..499->4, 500..599->5, else 0.`
- **Signature Hint**: `f http_status_family (code) [`
- **Test Call (pIR)**: `p (http_status_family 404)`
- **Test Call (.psudo)**: `print http_status_family(404)`
- **Expected Stdout**: `4`
```pir
f http_status_family (code) [
  ? (>= code 200) [
    ? (< code 300) [
      r 2
    ]
  ]
  ? (>= code 400) [
    ? (< code 500) [
      r 4
    ]
  ]
  ? (>= code 500) [
    ? (< code 600) [
      r 5
    ]
  ]
  r 0
]
```

### TC-02-09: Credit Tier
- **Category**: `CAT-02`
- **Prompt**: `Implement credit_tier(score): >=750 returns 3, >=670 returns 2, >=580 returns 1, else 0.`
- **Signature Hint**: `f credit_tier (score) [`
- **Test Call (pIR)**: `p (credit_tier 765)`
- **Test Call (.psudo)**: `print credit_tier(765)`
- **Expected Stdout**: `3`
```pir
f credit_tier (score) [
  ? (>= score 750) [
    r 3
  ]
  ? (>= score 670) [
    r 2
  ]
  ? (>= score 580) [
    r 1
  ]
  r 0
]
```

### TC-02-10: Sensor Alert Level
- **Category**: `CAT-02`
- **Prompt**: `Write sensor helper temp_alert_level(celsius): >=40 returns 2, <=0 returns -1, else 0.`
- **Signature Hint**: `f temp_alert_level (celsius) [`
- **Test Call (pIR)**: `p (temp_alert_level 43)`
- **Test Call (.psudo)**: `print temp_alert_level(43)`
- **Expected Stdout**: `2`
```pir
f temp_alert_level (celsius) [
  ? (>= celsius 40) [
    r 2
  ]
  ? (<= celsius 0) [
    r -1
  ]
  r 0
]
```


## CAT-03: Unbounded while Loops, Accumulators & mu-Minimization

### TC-03-01: Sum One to N
- **Category**: `CAT-03`
- **Prompt**: `Write sum_one_to_n(limit_n) using a while loop to sum integers from 1 through limit_n.`
- **Signature Hint**: `f sum_one_to_n (limit_n) [`
- **Test Call (pIR)**: `p (sum_one_to_n 10)`
- **Test Call (.psudo)**: `print sum_one_to_n(10)`
- **Expected Stdout**: `55`
```pir
f sum_one_to_n (limit_n) [
  v acc 0
  v idx 1
  w (<= idx limit_n) [
    = acc (+ acc idx)
    = idx (+ idx 1)
  ]
  r acc
]
```

### TC-03-02: Iterative Factorial
- **Category**: `CAT-03`
- **Prompt**: `Implement iterative factorial_iter(num_n) multiplying 1..num_n in a loop.`
- **Signature Hint**: `f factorial_iter (num_n) [`
- **Test Call (pIR)**: `p (factorial_iter 6)`
- **Test Call (.psudo)**: `print factorial_iter(6)`
- **Expected Stdout**: `720`
```pir
f factorial_iter (num_n) [
  v prod 1
  v idx 2
  w (<= idx num_n) [
    = prod (* prod idx)
    = idx (+ idx 1)
  ]
  r prod
]
```

### TC-03-03: Integer Power
- **Category**: `CAT-03`
- **Prompt**: `Create int_power(base_v, exp_v) computing base_v raised to integer exponent exp_v.`
- **Signature Hint**: `f int_power (base_v exp_v) [`
- **Test Call (pIR)**: `p (int_power 3 5)`
- **Test Call (.psudo)**: `print int_power(3, 5)`
- **Expected Stdout**: `243`
```pir
f int_power (base_v exp_v) [
  v res 1
  v step 0
  w (< step exp_v) [
    = res (* res base_v)
    = step (+ step 1)
  ]
  r res
]
```

### TC-03-04: Integer Remainder via Subtraction
- **Category**: `CAT-03`
- **Prompt**: `Write int_mod_sub(num_a, div_b) finding the integer remainder via repeated subtraction.`
- **Signature Hint**: `f int_mod_sub (num_a div_b) [`
- **Test Call (pIR)**: `p (int_mod_sub 29 6)`
- **Test Call (.psudo)**: `print int_mod_sub(29, 6)`
- **Expected Stdout**: `5`
```pir
f int_mod_sub (num_a div_b) [
  v rem num_a
  w (>= rem div_b) [
    = rem (- rem div_b)
  ]
  r rem
]
```

### TC-03-05: Integer Quotient via Subtraction
- **Category**: `CAT-03`
- **Prompt**: `Write int_div_sub(num_a, div_b) computing integer floor quotient via repeated subtraction.`
- **Signature Hint**: `f int_div_sub (num_a div_b) [`
- **Test Call (pIR)**: `p (int_div_sub 29 6)`
- **Test Call (.psudo)**: `print int_div_sub(29, 6)`
- **Expected Stdout**: `4`
```pir
f int_div_sub (num_a div_b) [
  v rem num_a
  v quot 0
  w (>= rem div_b) [
    = rem (- rem div_b)
    = quot (+ quot 1)
  ]
  r quot
]
```

### TC-03-06: Count Decimal Digits
- **Category**: `CAT-03`
- **Prompt**: `How do I count base-10 digits of a positive integer in count_decimal_digits(num_n)?`
- **Signature Hint**: `f count_decimal_digits (num_n) [`
- **Test Call (pIR)**: `p (count_decimal_digits 48291)`
- **Test Call (.psudo)**: `print count_decimal_digits(48291)`
- **Expected Stdout**: `5`
```pir
f count_decimal_digits (num_n) [
  v cur num_n
  v digits 1
  w (>= cur 10) [
    = cur (/ cur 10)
    = digits (+ digits 1)
  ]
  r digits
]
```

### TC-03-07: Arithmetic Series Sum
- **Category**: `CAT-03`
- **Prompt**: `Implement sum_arith_series(start_v, step_v, terms) summing an arithmetic progression.`
- **Signature Hint**: `f sum_arith_series (start_v step_v terms) [`
- **Test Call (pIR)**: `p (sum_arith_series 2 3 4)`
- **Test Call (.psudo)**: `print sum_arith_series(2, 3, 4)`
- **Expected Stdout**: `26`
```pir
f sum_arith_series (start_v step_v terms) [
  v total 0
  v cur start_v
  v idx 0
  w (< idx terms) [
    = total (+ total cur)
    = cur (+ cur step_v)
    = idx (+ idx 1)
  ]
  r total
]
```

### TC-03-08: Geometric Sum
- **Category**: `CAT-03`
- **Prompt**: `Write geometric_sum(ratio, terms) computing 1 + ratio + ratio^2 + ... for terms elements.`
- **Signature Hint**: `f geometric_sum (ratio terms) [`
- **Test Call (pIR)**: `p (geometric_sum 2 5)`
- **Test Call (.psudo)**: `print geometric_sum(2, 5)`
- **Expected Stdout**: `31`
```pir
f geometric_sum (ratio terms) [
  v total 0
  v term 1
  v idx 0
  w (< idx terms) [
    = total (+ total term)
    = term (* term ratio)
    = idx (+ idx 1)
  ]
  r total
]
```

### TC-03-09: Integer Square Root Floor
- **Category**: `CAT-03`
- **Prompt**: `fun to find integer floor square root isqrt_floor(num_n) without builtins`
- **Signature Hint**: `f isqrt_floor (num_n) [`
- **Test Call (pIR)**: `p (isqrt_floor 50)`
- **Test Call (.psudo)**: `print isqrt_floor(50)`
- **Expected Stdout**: `7`
```pir
f isqrt_floor (num_n) [
  v root 0
  w (<= (* (+ root 1) (+ root 1)) num_n) [
    = root (+ root 1)
  ]
  r root
]
```

### TC-03-10: Years to Target Balance
- **Category**: `CAT-03`
- **Prompt**: `Write years_to_target(start_bal, annual_add, target_bal) counting years until balance reaches target.`
- **Signature Hint**: `f years_to_target (start_bal annual_add target_bal) [`
- **Test Call (pIR)**: `p (years_to_target 100 35 250)`
- **Test Call (.psudo)**: `print years_to_target(100, 35, 250)`
- **Expected Stdout**: `5`
```pir
f years_to_target (start_bal annual_add target_bal) [
  v bal start_bal
  v yrs 0
  w (< bal target_bal) [
    = bal (+ bal annual_add)
    = yrs (+ yrs 1)
  ]
  r yrs
]
```


## CAT-04: General Recursion, Ackermann, Collatz & Universal Turing Machines

### TC-04-01: Recursive Fibonacci
- **Category**: `CAT-04`
- **Prompt**: `Write recursive Fibonacci function fib_rec(num_n) returning the nth Fibonacci number.`
- **Signature Hint**: `f fib_rec (num_n) [`
- **Test Call (pIR)**: `p (fib_rec 10)`
- **Test Call (.psudo)**: `print fib_rec(10)`
- **Expected Stdout**: `55`
```pir
f fib_rec (num_n) [
  ? (<= num_n 1) [
    r num_n
  ]
  r (+ (fib_rec (- num_n 1)) (fib_rec (- num_n 2)))
]
```

### TC-04-02: Recursive Factorial
- **Category**: `CAT-04`
- **Prompt**: `Implement recursive factorial fact_rec(num_n) with base case num_n <= 1 returning 1.`
- **Signature Hint**: `f fact_rec (num_n) [`
- **Test Call (pIR)**: `p (fact_rec 5)`
- **Test Call (.psudo)**: `print fact_rec(5)`
- **Expected Stdout**: `120`
```pir
f fact_rec (num_n) [
  ? (<= num_n 1) [
    r 1
  ]
  r (* num_n (fact_rec (- num_n 1)))
]
```

### TC-04-03: Recursive Euclidean GCD
- **Category**: `CAT-04`
- **Prompt**: `Implement recursive Euclidean GCD gcd_rec(pa, pb) for two positive integers.`
- **Signature Hint**: `f gcd_rec (pa pb) [`
- **Test Call (pIR)**: `p (gcd_rec 48 18)`
- **Test Call (.psudo)**: `print gcd_rec(48, 18)`
- **Expected Stdout**: `6`
```pir
f gcd_rec (pa pb) [
  ? (== pa pb) [
    r pa
  ]
  ? (> pa pb) [
    r (gcd_rec (- pa pb) pb)
  ]
  r (gcd_rec pa (- pb pa))
]
```

### TC-04-04: Ackermann-Peter Function
- **Category**: `CAT-04`
- **Prompt**: `Write the non-primitive recursive Ackermann-Peter function ackermann(am, an).`
- **Signature Hint**: `f ackermann (am an) [`
- **Test Call (pIR)**: `p (ackermann 2 2)`
- **Test Call (.psudo)**: `print ackermann(2, 2)`
- **Expected Stdout**: `7`
```pir
f ackermann (am an) [
  ? (== am 0) [
    r (+ an 1)
  ]
  ? (== an 0) [
    r (ackermann (- am 1) 1)
  ]
  r (ackermann (- am 1) (ackermann am (- an 1)))
]
```

### TC-04-05: Collatz Hailstone Steps
- **Category**: `CAT-04`
- **Prompt**: `How do I write collatz_steps(start_n) counting hailstone steps until start_n reaches 1?`
- **Signature Hint**: `f collatz_steps (start_n) [`
- **Test Call (pIR)**: `p (collatz_steps 6)`
- **Test Call (.psudo)**: `print collatz_steps(6)`
- **Expected Stdout**: `8`
```pir
f collatz_steps (start_n) [
  v cur start_n
  v steps 0
  w (> cur 1) [
    v rem cur
    v half 0
    w (>= rem 2) [
      = rem (- rem 2)
      = half (+ half 1)
    ]
    ? (== rem 0) [
      = cur half
    ] [
      = cur (+ (* cur 3) 1)
    ]
    = steps (+ steps 1)
  ]
  r steps
]
```

### TC-04-06: Minsky Register Machine
- **Category**: `CAT-04`
- **Prompt**: `Simulate a 2-counter Minsky register machine minsky_transfer_double(reg_a, reg_b) that decrements reg_a to zero while incrementing reg_b twice per step.`
- **Signature Hint**: `f minsky_transfer_double (reg_a reg_b) [`
- **Test Call (pIR)**: `p (minsky_transfer_double 7 3)`
- **Test Call (.psudo)**: `print minsky_transfer_double(7, 3)`
- **Expected Stdout**: `17`
```pir
f minsky_transfer_double (reg_a reg_b) [
  v ra reg_a
  v rb reg_b
  w (> ra 0) [
    = ra (- ra 1)
    = rb (+ rb 1)
    = rb (+ rb 1)
  ]
  r rb
]
```

### TC-04-07: Rule 110 Cellular Automaton Cell
- **Category**: `CAT-04`
- **Prompt**: `Implement Elementary Cellular Automaton Rule 110 transition rule110_cell(left_c, mid_c, right_c) returning 0 or 1.`
- **Signature Hint**: `f rule110_cell (left_c mid_c right_c) [`
- **Test Call (pIR)**: `p (rule110_cell 1 1 0)`
- **Test Call (.psudo)**: `print rule110_cell(1, 1, 0)`
- **Expected Stdout**: `1`
```pir
f rule110_cell (left_c mid_c right_c) [
  v pat (+ (+ (* left_c 4) (* mid_c 2)) right_c)
  ? (== pat 7) [
    r 0
  ]
  ? (== pat 4) [
    r 0
  ]
  ? (== pat 0) [
    r 0
  ]
  r 1
]
```

### TC-04-08: Recursive Tribonacci
- **Category**: `CAT-04`
- **Prompt**: `Write recursive tribonacci_rec(num_n) where T(0)=0, T(1)=1, T(2)=1, and T(n)=T(n-1)+T(n-2)+T(n-3).`
- **Signature Hint**: `f tribonacci_rec (num_n) [`
- **Test Call (pIR)**: `p (tribonacci_rec 6)`
- **Test Call (.psudo)**: `print tribonacci_rec(6)`
- **Expected Stdout**: `13`
```pir
f tribonacci_rec (num_n) [
  ? (== num_n 0) [
    r 0
  ]
  ? (<= num_n 2) [
    r 1
  ]
  v t1 (tribonacci_rec (- num_n 1))
  v t2 (tribonacci_rec (- num_n 2))
  v t3 (tribonacci_rec (- num_n 3))
  r (+ (+ t1 t2) t3)
]
```

### TC-04-09: Tower of Hanoi Moves
- **Category**: `CAT-04`
- **Prompt**: `Compute minimum Tower of Hanoi moves recursively in hanoi_moves(disks).`
- **Signature Hint**: `f hanoi_moves (disks) [`
- **Test Call (pIR)**: `p (hanoi_moves 4)`
- **Test Call (.psudo)**: `print hanoi_moves(4)`
- **Expected Stdout**: `15`
```pir
f hanoi_moves (disks) [
  ? (<= disks 1) [
    r disks
  ]
  r (+ (* 2 (hanoi_moves (- disks 1))) 1)
]
```

### TC-04-10: McCarthy 91 Function
- **Category**: `CAT-04`
- **Prompt**: `Implement John McCarthy's nested recursive 91 function mccarthy91(num_n).`
- **Signature Hint**: `f mccarthy91 (num_n) [`
- **Test Call (pIR)**: `p (mccarthy91 95)`
- **Test Call (.psudo)**: `print mccarthy91(95)`
- **Expected Stdout**: `91`
```pir
f mccarthy91 (num_n) [
  ? (> num_n 100) [
    r (- num_n 10)
  ]
  r (mccarthy91 (mccarthy91 (+ num_n 11)))
]
```


## CAT-05: Number Theory, Primality & Bitwise-Equivalent Integer Encodings

### TC-05-01: Is Even Integer
- **Category**: `CAT-05`
- **Prompt**: `Write is_even_int(num_n) returning true if non-negative integer num_n is even, else false.`
- **Signature Hint**: `f is_even_int (num_n) [`
- **Test Call (pIR)**: `p (is_even_int 14)`
- **Test Call (.psudo)**: `print is_even_int(14)`
- **Expected Stdout**: `true`
```pir
f is_even_int (num_n) [
  v rem num_n
  w (>= rem 2) [
    = rem (- rem 2)
  ]
  r (== rem 0)
]
```

### TC-05-02: Is Odd Integer
- **Category**: `CAT-05`
- **Prompt**: `Write is_odd_int(num_n) returning true when non-negative integer num_n is odd.`
- **Signature Hint**: `f is_odd_int (num_n) [`
- **Test Call (pIR)**: `p (is_odd_int 15)`
- **Test Call (.psudo)**: `print is_odd_int(15)`
- **Expected Stdout**: `true`
```pir
f is_odd_int (num_n) [
  v rem num_n
  w (>= rem 2) [
    = rem (- rem 2)
  ]
  r (== rem 1)
]
```

### TC-05-03: Trial Division Primality Test
- **Category**: `CAT-05`
- **Prompt**: `Implement primality test is_prime_num(num_n) using trial division up to sqrt(num_n).`
- **Signature Hint**: `f is_prime_num (num_n) [`
- **Test Call (pIR)**: `p (is_prime_num 17)`
- **Test Call (.psudo)**: `print is_prime_num(17)`
- **Expected Stdout**: `true`
```pir
f is_prime_num (num_n) [
  ? (< num_n 2) [
    r false
  ]
  v div 2
  w (<= (* div div) num_n) [
    v rem num_n
    w (>= rem div) [
      = rem (- rem div)
    ]
    ? (== rem 0) [
      r false
    ]
    = div (+ div 1)
  ]
  r true
]
```

### TC-05-04: Count Positive Divisors
- **Category**: `CAT-05`
- **Prompt**: `Count how many positive divisors num_n has in count_divisors(num_n).`
- **Signature Hint**: `f count_divisors (num_n) [`
- **Test Call (pIR)**: `p (count_divisors 12)`
- **Test Call (.psudo)**: `print count_divisors(12)`
- **Expected Stdout**: `6`
```pir
f count_divisors (num_n) [
  v cnt 0
  v div 1
  w (<= div num_n) [
    v rem num_n
    w (>= rem div) [
      = rem (- rem div)
    ]
    ? (== rem 0) [
      = cnt (+ cnt 1)
    ]
    = div (+ div 1)
  ]
  r cnt
]
```

### TC-05-05: Sum of Divisors
- **Category**: `CAT-05`
- **Prompt**: `Write sum_divisors(num_n) returning the sum of all positive divisors of num_n.`
- **Signature Hint**: `f sum_divisors (num_n) [`
- **Test Call (pIR)**: `p (sum_divisors 12)`
- **Test Call (.psudo)**: `print sum_divisors(12)`
- **Expected Stdout**: `28`
```pir
f sum_divisors (num_n) [
  v total 0
  v div 1
  w (<= div num_n) [
    v rem num_n
    w (>= rem div) [
      = rem (- rem div)
    ]
    ? (== rem 0) [
      = total (+ total div)
    ]
    = div (+ div 1)
  ]
  r total
]
```

### TC-05-06: Least Common Multiple
- **Category**: `CAT-05`
- **Prompt**: `Create lcm_two(pa, pb) returning the least common multiple of two positive integers.`
- **Signature Hint**: `f lcm_two (pa pb) [`
- **Test Call (pIR)**: `p (lcm_two 12 18)`
- **Test Call (.psudo)**: `print lcm_two(12, 18)`
- **Expected Stdout**: `36`
```pir
f lcm_two (pa pb) [
  v ga pa
  v gb pb
  w (!= ga gb) [
    ? (> ga gb) [
      = ga (- ga gb)
    ] [
      = gb (- gb ga)
    ]
  ]
  r (/ (* pa pb) ga)
]
```

### TC-05-07: Nth Prime Number
- **Category**: `CAT-05`
- **Prompt**: `Find the kth prime number (1-indexed, so k=1 is 2) in nth_prime(target_k).`
- **Signature Hint**: `f nth_prime (target_k) [`
- **Test Call (pIR)**: `p (nth_prime 5)`
- **Test Call (.psudo)**: `print nth_prime(5)`
- **Expected Stdout**: `11`
```pir
f nth_prime (target_k) [
  v found 0
  v cand 1
  w (< found target_k) [
    = cand (+ cand 1)
    v div 2
    v prime_ok true
    w (<= (* div div) cand) [
      v rem cand
      w (>= rem div) [
        = rem (- rem div)
      ]
      ? (== rem 0) [
        = prime_ok false
      ]
      = div (+ div 1)
    ]
    ? prime_ok [
      = found (+ found 1)
    ]
  ]
  r cand
]
```

### TC-05-08: Bitwise Left Shift by Power of Two
- **Category**: `CAT-05`
- **Prompt**: `Simulate bitwise left shift shl_pow2(val, bits) multiplying val by 2^bits.`
- **Signature Hint**: `f shl_pow2 (val bits) [`
- **Test Call (pIR)**: `p (shl_pow2 5 3)`
- **Test Call (.psudo)**: `print shl_pow2(5, 3)`
- **Expected Stdout**: `40`
```pir
f shl_pow2 (val bits) [
  v out val
  v idx 0
  w (< idx bits) [
    = out (* out 2)
    = idx (+ idx 1)
  ]
  r out
]
```

### TC-05-09: Unsigned Right Shift Floor Division
- **Category**: `CAT-05`
- **Prompt**: `Simulate unsigned right shift shr_floor(val, bits) floor-dividing by 2 bits times.`
- **Signature Hint**: `f shr_floor (val bits) [`
- **Test Call (pIR)**: `p (shr_floor 45 2)`
- **Test Call (.psudo)**: `print shr_floor(45, 2)`
- **Expected Stdout**: `11`
```pir
f shr_floor (val bits) [
  v cur val
  v idx 0
  w (< idx bits) [
    v rem cur
    v half 0
    w (>= rem 2) [
      = rem (- rem 2)
      = half (+ half 1)
    ]
    = cur half
    = idx (+ idx 1)
  ]
  r cur
]
```

### TC-05-10: Population Count (Set Bits)
- **Category**: `CAT-05`
- **Prompt**: `Write popcount_bits(num_n) counting set 1-bits in the binary representation of num_n.`
- **Signature Hint**: `f popcount_bits (num_n) [`
- **Test Call (pIR)**: `p (popcount_bits 29)`
- **Test Call (.psudo)**: `print popcount_bits(29)`
- **Expected Stdout**: `4`
```pir
f popcount_bits (num_n) [
  v cur num_n
  v ones 0
  w (> cur 0) [
    v rem cur
    v half 0
    w (>= rem 2) [
      = rem (- rem 2)
      = half (+ half 1)
    ]
    = ones (+ ones rem)
    = cur half
  ]
  r ones
]
```


## CAT-06: Strings, Text Formatting, Validation & Log Redaction

### TC-06-01: Greet User
- **Category**: `CAT-06`
- **Prompt**: `Write greet_user(user_name) returning "Hello, <user_name>!".`
- **Signature Hint**: `f greet_user (user_name) [`
- **Test Call (pIR)**: `p (greet_user "Ada")`
- **Test Call (.psudo)**: `print greet_user("Ada")`
- **Expected Stdout**: `Hello, Ada!`
```pir
f greet_user (user_name) [
  r (+ (+ "Hello, " user_name) "!")
]
```

### TC-06-02: Join URL Path
- **Category**: `CAT-06`
- **Prompt**: `Join a base URL and route segment with a slash in join_url_path(base_url, segment).`
- **Signature Hint**: `f join_url_path (base_url segment) [`
- **Test Call (pIR)**: `p (join_url_path "https://api.local" "v1")`
- **Test Call (.psudo)**: `print join_url_path("https://api.local", "v1")`
- **Expected Stdout**: `https://api.local/v1`
```pir
f join_url_path (base_url segment) [
  r (+ (+ base_url "/") segment)
]
```

### TC-06-03: Wrap XML Tag
- **Category**: `CAT-06`
- **Prompt**: `Create wrap_xml_tag(tag_name, body_txt) wrapping body_txt inside <tag_name>...</tag_name>.`
- **Signature Hint**: `f wrap_xml_tag (tag_name body_txt) [`
- **Test Call (pIR)**: `p (wrap_xml_tag "code" "ok")`
- **Test Call (.psudo)**: `print wrap_xml_tag("code", "ok")`
- **Expected Stdout**: `<code>ok</code>`
```pir
f wrap_xml_tag (tag_name body_txt) [
  v open_t (+ (+ "<" tag_name) ">")
  v close_t (+ (+ "</" tag_name) ">")
  r (+ (+ open_t body_txt) close_t)
]
```

### TC-06-04: Format Dotenv Pair
- **Category**: `CAT-06`
- **Prompt**: `Format a dotenv line KEY=VALUE in format_env_pair(key_str, val_str).`
- **Signature Hint**: `f format_env_pair (key_str val_str) [`
- **Test Call (pIR)**: `p (format_env_pair "PORT" "8080")`
- **Test Call (.psudo)**: `print format_env_pair("PORT", "8080")`
- **Expected Stdout**: `PORT=8080`
```pir
f format_env_pair (key_str val_str) [
  r (+ (+ key_str "=") val_str)
]
```

### TC-06-05: Format Log Line
- **Category**: `CAT-06`
- **Prompt**: `Write format_log_line(level_str, msg_str) producing "[LEVEL] msg".`
- **Signature Hint**: `f format_log_line (level_str msg_str) [`
- **Test Call (pIR)**: `p (format_log_line "WARN" "disk full")`
- **Test Call (.psudo)**: `print format_log_line("WARN", "disk full")`
- **Expected Stdout**: `[WARN] disk full`
```pir
f format_log_line (level_str msg_str) [
  r (+ (+ (+ "[" level_str) "] ") msg_str)
]
```

### TC-06-06: Minimum Password Length Check
- **Category**: `CAT-06`
- **Prompt**: `Check password minimum length in is_strong_len(password_str, min_chars) using len().`
- **Signature Hint**: `f is_strong_len (password_str min_chars) [`
- **Test Call (pIR)**: `p (is_strong_len "s3cr3t_p4ssw0rd" 12)`
- **Test Call (.psudo)**: `print is_strong_len("s3cr3t_p4ssw0rd", 12)`
- **Expected Stdout**: `true`
```pir
f is_strong_len (password_str min_chars) [
  r (>= (len password_str) min_chars)
]
```

### TC-06-07: Bearer Authorization Header
- **Category**: `CAT-06`
- **Prompt**: `Build HTTP Authorization header value "Bearer <tok_str>" in bearer_auth_header(tok_str).`
- **Signature Hint**: `f bearer_auth_header (tok_str) [`
- **Test Call (pIR)**: `p (bearer_auth_header "xyz987")`
- **Test Call (.psudo)**: `print bearer_auth_header("xyz987")`
- **Expected Stdout**: `Bearer xyz987`
```pir
f bearer_auth_header (tok_str) [
  r (+ "Bearer " tok_str)
]
```

### TC-06-08: Build CSV Row
- **Category**: `CAT-06`
- **Prompt**: `Write build_csv_row(col1, col2, col3) joining three fields with commas.`
- **Signature Hint**: `f build_csv_row (col1 col2 col3) [`
- **Test Call (pIR)**: `p (build_csv_row "alice" "admin" "active")`
- **Test Call (.psudo)**: `print build_csv_row("alice", "admin", "active")`
- **Expected Stdout**: `alice,admin,active`
```pir
f build_csv_row (col1 col2 col3) [
  r (+ (+ (+ (+ col1 ",") col2) ",") col3)
]
```

### TC-06-09: Repeat Token String
- **Category**: `CAT-06`
- **Prompt**: `Repeat unit_str exactly times_n times using a loop in repeat_token(unit_str, times_n).`
- **Signature Hint**: `f repeat_token (unit_str times_n) [`
- **Test Call (pIR)**: `p (repeat_token "ha" 3)`
- **Test Call (.psudo)**: `print repeat_token("ha", 3)`
- **Expected Stdout**: `hahaha`
```pir
f repeat_token (unit_str times_n) [
  v out ""
  v idx 0
  w (< idx times_n) [
    = out (+ out unit_str)
    = idx (+ idx 1)
  ]
  r out
]
```

### TC-06-10: Coalesce String Fallback
- **Category**: `CAT-06`
- **Prompt**: `Write coalesce_str(primary_s, fallback_s) returning fallback_s when primary_s is empty.`
- **Signature Hint**: `f coalesce_str (primary_s fallback_s) [`
- **Test Call (pIR)**: `p (coalesce_str "" "default_cfg")`
- **Test Call (.psudo)**: `print coalesce_str("", "default_cfg")`
- **Expected Stdout**: `default_cfg`
```pir
f coalesce_str (primary_s fallback_s) [
  ? (== primary_s "") [
    r fallback_s
  ]
  r primary_s
]
```


## CAT-07: E-Commerce, FinTech, Billing & SaaS Business Rules

### TC-07-01: Tiered SaaS Seat Billing
- **Category**: `CAT-07`
- **Prompt**: `Tiered SaaS billing saas_seat_bill(seats): first 5 seats cost 20 each, additional seats 15 each.`
- **Signature Hint**: `f saas_seat_bill (seats) [`
- **Test Call (pIR)**: `p (saas_seat_bill 8)`
- **Test Call (.psudo)**: `print saas_seat_bill(8)`
- **Expected Stdout**: `145`
```pir
f saas_seat_bill (seats) [
  ? (<= seats 5) [
    r (* seats 20)
  ]
  r (+ 100 (* (- seats 5) 15))
]
```

### TC-07-02: Overtime Pay Calculation
- **Category**: `CAT-07`
- **Prompt**: `Payroll helper overtime_pay(hours, hourly_rate): 1.5x rate for hours worked above 40.`
- **Signature Hint**: `f overtime_pay (hours hourly_rate) [`
- **Test Call (pIR)**: `p (overtime_pay 46 20)`
- **Test Call (.psudo)**: `print overtime_pay(46, 20)`
- **Expected Stdout**: `980`
```pir
f overtime_pay (hours hourly_rate) [
  ? (<= hours 40) [
    r (* hours hourly_rate)
  ]
  v base_pay (* 40 hourly_rate)
  v extra (/ (* (* (- hours 40) hourly_rate) 3) 2)
  r (+ base_pay extra)
]
```

### TC-07-03: Promotional Cart Discount
- **Category**: `CAT-07`
- **Prompt**: `Write promo_cart_total(cart_amt, min_spend, coupon_off) applying coupon_off if cart_amt >= min_spend.`
- **Signature Hint**: `f promo_cart_total (cart_amt min_spend coupon_off) [`
- **Test Call (pIR)**: `p (promo_cart_total 120 100 25)`
- **Test Call (.psudo)**: `print promo_cart_total(120, 100, 25)`
- **Expected Stdout**: `95`
```pir
f promo_cart_total (cart_amt min_spend coupon_off) [
  ? (>= cart_amt min_spend) [
    r (- cart_amt coupon_off)
  ]
  r cart_amt
]
```

### TC-07-04: Prorated Subscription Refund
- **Category**: `CAT-07`
- **Prompt**: `Calculate 30-day prorated subscription refund in prorated_refund(monthly_fee, days_unused).`
- **Signature Hint**: `f prorated_refund (monthly_fee days_unused) [`
- **Test Call (pIR)**: `p (prorated_refund 90 10)`
- **Test Call (.psudo)**: `print prorated_refund(90, 10)`
- **Expected Stdout**: `30`
```pir
f prorated_refund (monthly_fee days_unused) [
  r (/ (* monthly_fee days_unused) 30)
]
```

### TC-07-05: Loyalty Points Calculation
- **Category**: `CAT-07`
- **Prompt**: `Compute loyalty_points(dollars_spent, is_vip): VIPs earn 3x points, standard members 1x.`
- **Signature Hint**: `f loyalty_points (dollars_spent is_vip) [`
- **Test Call (pIR)**: `p (loyalty_points 80 true)`
- **Test Call (.psudo)**: `print loyalty_points(80, true)`
- **Expected Stdout**: `240`
```pir
f loyalty_points (dollars_spent is_vip) [
  ? is_vip [
    r (* dollars_spent 3)
  ]
  r dollars_spent
]
```

### TC-07-06: Compound Late Invoice Fee
- **Category**: `CAT-07`
- **Prompt**: `Write compound_late_invoice(amount, months_late, fee_per_month) adding linear monthly late fees.`
- **Signature Hint**: `f compound_late_invoice (amount months_late fee_per_month) [`
- **Test Call (pIR)**: `p (compound_late_invoice 500 3 25)`
- **Test Call (.psudo)**: `print compound_late_invoice(500, 3, 25)`
- **Expected Stdout**: `575`
```pir
f compound_late_invoice (amount months_late fee_per_month) [
  r (+ amount (* months_late fee_per_month))
]
```

### TC-07-07: Warehouse Reorder Quantity
- **Category**: `CAT-07`
- **Prompt**: `Warehouse helper reorder_quantity(stock_now, reorder_pt, target_stock): order up to target_stock when stock_now <= reorder_pt, else 0.`
- **Signature Hint**: `f reorder_quantity (stock_now reorder_pt target_stock) [`
- **Test Call (pIR)**: `p (reorder_quantity 15 20 100)`
- **Test Call (.psudo)**: `print reorder_quantity(15, 20, 100)`
- **Expected Stdout**: `85`
```pir
f reorder_quantity (stock_now reorder_pt target_stock) [
  ? (<= stock_now reorder_pt) [
    r (- target_stock stock_now)
  ]
  r 0
]
```

### TC-07-08: Foreign Exchange Net Conversion
- **Category**: `CAT-07`
- **Prompt**: `Convert USD currency in fx_convert_net(usd_amt, rate_x100, flat_fee) as (usd_amt * rate_x100 / 100) - flat_fee.`
- **Signature Hint**: `f fx_convert_net (usd_amt rate_x100 flat_fee) [`
- **Test Call (pIR)**: `p (fx_convert_net 200 92 4)`
- **Test Call (.psudo)**: `print fx_convert_net(200, 92, 4)`
- **Expected Stdout**: `180`
```pir
f fx_convert_net (usd_amt rate_x100 flat_fee) [
  r (- (/ (* usd_amt rate_x100) 100) flat_fee)
]
```

### TC-07-09: Debt-to-Income Underwriting Check
- **Category**: `CAT-07`
- **Prompt**: `Underwriting check debt_to_income_ok(monthly_debt, monthly_inc): true when debt is at most 36% of income.`
- **Signature Hint**: `f debt_to_income_ok (monthly_debt monthly_inc) [`
- **Test Call (pIR)**: `p (debt_to_income_ok 1500 5000)`
- **Test Call (.psudo)**: `print debt_to_income_ok(1500, 5000)`
- **Expected Stdout**: `true`
```pir
f debt_to_income_ok (monthly_debt monthly_inc) [
  r (<= (* monthly_debt 100) (* monthly_inc 36))
]
```

### TC-07-10: Annual Recurring Revenue
- **Category**: `CAT-07`
- **Prompt**: `Compute annualized recurring revenue annual_arr(mrr_val, expansion_mrr, churn_mrr).`
- **Signature Hint**: `f annual_arr (mrr_val expansion_mrr churn_mrr) [`
- **Test Call (pIR)**: `p (annual_arr 1000 250 50)`
- **Test Call (.psudo)**: `print annual_arr(1000, 250, 50)`
- **Expected Stdout**: `14400`
```pir
f annual_arr (mrr_val expansion_mrr churn_mrr) [
  v net_mrr (- (+ mrr_val expansion_mrr) churn_mrr)
  r (* net_mrr 12)
]
```


## CAT-08: Backend Infrastructure, Rate Limiting, Auth & SRE Guardrails

### TC-08-01: Token Bucket Consume
- **Category**: `CAT-08`
- **Prompt**: `Implement token_bucket_consume(avail_toks, req_cost) returning remaining tokens or -1 if insufficient.`
- **Signature Hint**: `f token_bucket_consume (avail_toks req_cost) [`
- **Test Call (pIR)**: `p (token_bucket_consume 50 12)`
- **Test Call (.psudo)**: `print token_bucket_consume(50, 12)`
- **Expected Stdout**: `38`
```pir
f token_bucket_consume (avail_toks req_cost) [
  ? (>= avail_toks req_cost) [
    r (- avail_toks req_cost)
  ]
  r -1
]
```

### TC-08-02: Session TTL Expiration Check
- **Category**: `CAT-08`
- **Prompt**: `Session TTL check is_session_valid(last_seen, ttl_sec, now_sec) returning true if elapsed < ttl_sec.`
- **Signature Hint**: `f is_session_valid (last_seen ttl_sec now_sec) [`
- **Test Call (pIR)**: `p (is_session_valid 1000 300 1250)`
- **Test Call (.psudo)**: `print is_session_valid(1000, 300, 1250)`
- **Expected Stdout**: `true`
```pir
f is_session_valid (last_seen ttl_sec now_sec) [
  r (< (- now_sec last_seen) ttl_sec)
]
```

### TC-08-03: Exponential Retry Backoff Delay
- **Category**: `CAT-08`
- **Prompt**: `Write exponential retry delay exp_backoff_ms(base_ms, attempt_n) computing base_ms * 2^attempt_n.`
- **Signature Hint**: `f exp_backoff_ms (base_ms attempt_n) [`
- **Test Call (pIR)**: `p (exp_backoff_ms 100 4)`
- **Test Call (.psudo)**: `print exp_backoff_ms(100, 4)`
- **Expected Stdout**: `1600`
```pir
f exp_backoff_ms (base_ms attempt_n) [
  v delay base_ms
  v idx 0
  w (< idx attempt_n) [
    = delay (* delay 2)
    = idx (+ idx 1)
  ]
  r delay
]
```

### TC-08-04: Circuit Breaker Trip Check
- **Category**: `CAT-08`
- **Prompt**: `Circuit breaker check circuit_is_open(err_count, trip_thresh): true when err_count >= trip_thresh.`
- **Signature Hint**: `f circuit_is_open (err_count trip_thresh) [`
- **Test Call (pIR)**: `p (circuit_is_open 7 5)`
- **Test Call (.psudo)**: `print circuit_is_open(7, 5)`
- **Expected Stdout**: `true`
```pir
f circuit_is_open (err_count trip_thresh) [
  r (>= err_count trip_thresh)
]
```

### TC-08-05: Raft Quorum Majority Check
- **Category**: `CAT-08`
- **Prompt**: `Raft consensus helper has_quorum_majority(yes_votes, total_nodes): true if yes_votes is strict majority.`
- **Signature Hint**: `f has_quorum_majority (yes_votes total_nodes) [`
- **Test Call (pIR)**: `p (has_quorum_majority 3 5)`
- **Test Call (.psudo)**: `print has_quorum_majority(3, 5)`
- **Expected Stdout**: `true`
```pir
f has_quorum_majority (yes_votes total_nodes) [
  r (> (* yes_votes 2) total_nodes)
]
```

### TC-08-06: DB Connection Pool Idle Count
- **Category**: `CAT-08`
- **Prompt**: `DB pool metric pool_idle_conns(max_conns, active_conns) clamped at 0 minimum.`
- **Signature Hint**: `f pool_idle_conns (max_conns active_conns) [`
- **Test Call (pIR)**: `p (pool_idle_conns 64 19)`
- **Test Call (.psudo)**: `print pool_idle_conns(64, 19)`
- **Expected Stdout**: `45`
```pir
f pool_idle_conns (max_conns active_conns) [
  ? (>= active_conns max_conns) [
    r 0
  ]
  r (- max_conns active_conns)
]
```

### TC-08-07: RBAC Role Access Guard
- **Category**: `CAT-08`
- **Prompt**: `Role guard rbac_can_access(actor_role, min_role) returning true if actor_role >= min_role.`
- **Signature Hint**: `f rbac_can_access (actor_role min_role) [`
- **Test Call (pIR)**: `p (rbac_can_access 3 2)`
- **Test Call (.psudo)**: `print rbac_can_access(3, 2)`
- **Expected Stdout**: `true`
```pir
f rbac_can_access (actor_role min_role) [
  r (>= actor_role min_role)
]
```

### TC-08-08: Pending Database Migrations
- **Category**: `CAT-08`
- **Prompt**: `Migration check pending_migrations(db_ver, target_ver) returning how many schema versions remain.`
- **Signature Hint**: `f pending_migrations (db_ver target_ver) [`
- **Test Call (pIR)**: `p (pending_migrations 12 17)`
- **Test Call (.psudo)**: `print pending_migrations(12, 17)`
- **Expected Stdout**: `5`
```pir
f pending_migrations (db_ver target_ver) [
  ? (> target_ver db_ver) [
    r (- target_ver db_ver)
  ]
  r 0
]
```

### TC-08-09: Dead Letter Queue Route Check
- **Category**: `CAT-08`
- **Prompt**: `Queue worker check route_to_dlq(retry_cnt, max_attempts) when retries hit max_attempts.`
- **Signature Hint**: `f route_to_dlq (retry_cnt max_attempts) [`
- **Test Call (pIR)**: `p (route_to_dlq 4 3)`
- **Test Call (.psudo)**: `print route_to_dlq(4, 3)`
- **Expected Stdout**: `true`
```pir
f route_to_dlq (retry_cnt max_attempts) [
  r (>= retry_cnt max_attempts)
]
```

### TC-08-10: SRE Disk Pressure State
- **Category**: `CAT-08`
- **Prompt**: `SRE alert disk_pressure_state(used_pct): >=95 critical (2), >=80 warning (1), else healthy (0).`
- **Signature Hint**: `f disk_pressure_state (used_pct) [`
- **Test Call (pIR)**: `p (disk_pressure_state 88)`
- **Test Call (.psudo)**: `print disk_pressure_state(88)`
- **Expected Stdout**: `1`
```pir
f disk_pressure_state (used_pct) [
  ? (>= used_pct 95) [
    r 2
  ]
  ? (>= used_pct 80) [
    r 1
  ]
  r 0
]
```


## CAT-09: Sandboxed Filesystem I/O & Result Error-Handling Monads

### TC-09-01: Write and Read Config File
- **Category**: `CAT-09`
- **Prompt**: `Write payload to file_name in the sandbox and read it back in write_and_read_cfg(file_name, payload).`
- **Signature Hint**: `f write_and_read_cfg (file_name payload) [`
- **Test Call (pIR)**: `p (write_and_read_cfg "tc09_01.txt" "mode=prod")`
- **Test Call (.psudo)**: `print write_and_read_cfg("tc09_01.txt", "mode=prod")`
- **Expected Stdout**: `mode=prod`
```pir
f write_and_read_cfg (file_name payload) [
  @wt file_name payload
  r (@rd file_name)
]
```

### TC-09-02: Write and Check Exists
- **Category**: `CAT-09`
- **Prompt**: `Write text_val to file_name and return exists(file_name) in write_and_check_exists(file_name, text_val).`
- **Signature Hint**: `f write_and_check_exists (file_name text_val) [`
- **Test Call (pIR)**: `p (write_and_check_exists "tc09_02.txt" "hello")`
- **Test Call (.psudo)**: `print write_and_check_exists("tc09_02.txt", "hello")`
- **Expected Stdout**: `true`
```pir
f write_and_check_exists (file_name text_val) [
  @wt file_name text_val
  r (@ex file_name)
]
```

### TC-09-03: Ensure Sandbox Directory
- **Category**: `CAT-09`
- **Prompt**: `Create sandbox directory dir_name and verify isDir(dir_name) in ensure_sandbox_dir(dir_name).`
- **Signature Hint**: `f ensure_sandbox_dir (dir_name) [`
- **Test Call (pIR)**: `p (ensure_sandbox_dir "tc09_dir03")`
- **Test Call (.psudo)**: `print ensure_sandbox_dir("tc09_dir03")`
- **Expected Stdout**: `true`
```pir
f ensure_sandbox_dir (dir_name) [
  @mk dir_name
  r (@id dir_name)
]
```

### TC-09-04: Verify Regular File
- **Category**: `CAT-09`
- **Prompt**: `Write "data" to file_name and check isFile(file_name) in verify_regular_file(file_name).`
- **Signature Hint**: `f verify_regular_file (file_name) [`
- **Test Call (pIR)**: `p (verify_regular_file "tc09_04.txt")`
- **Test Call (.psudo)**: `print verify_regular_file("tc09_04.txt")`
- **Expected Stdout**: `true`
```pir
f verify_regular_file (file_name) [
  @wt file_name "data"
  r (@if file_name)
]
```

### TC-09-05: Copy Sandbox File
- **Category**: `CAT-09`
- **Prompt**: `Seed src_path with seed_txt, copy its contents to dst_path, and return dst_path contents in copy_sandbox_file(src_path, dst_path, seed_txt).`
- **Signature Hint**: `f copy_sandbox_file (src_path dst_path seed_txt) [`
- **Test Call (pIR)**: `p (copy_sandbox_file "tc09_src.txt" "tc09_dst.txt" "copied_ok")`
- **Test Call (.psudo)**: `print copy_sandbox_file("tc09_src.txt", "tc09_dst.txt", "copied_ok")`
- **Expected Stdout**: `copied_ok`
```pir
f copy_sandbox_file (src_path dst_path seed_txt) [
  @wt src_path seed_txt
  v body (@rd src_path)
  @wt dst_path body
  r (@rd dst_path)
]
```

### TC-09-06: Append Sandbox Line
- **Category**: `CAT-09`
- **Prompt**: `Write first_s to file_name, append second_s by reading and writing back, and return final text in append_sandbox_line(file_name, first_s, second_s).`
- **Signature Hint**: `f append_sandbox_line (file_name first_s second_s) [`
- **Test Call (pIR)**: `p (append_sandbox_line "tc09_log.txt" "A:" "B")`
- **Test Call (.psudo)**: `print append_sandbox_line("tc09_log.txt", "A:", "B")`
- **Expected Stdout**: `A:B`
```pir
f append_sandbox_line (file_name first_s second_s) [
  @wt file_name first_s
  v prev (@rd file_name)
  @wt file_name (+ prev second_s)
  r (@rd file_name)
]
```

### TC-09-07: Init Default If Missing
- **Category**: `CAT-09`
- **Prompt**: `Initialize file_name with fallback_txt if missing and return its contents in init_default_if_missing(file_name, fallback_txt).`
- **Signature Hint**: `f init_default_if_missing (file_name fallback_txt) [`
- **Test Call (pIR)**: `p (init_default_if_missing "tc09_init07.txt" "boot=1")`
- **Test Call (.psudo)**: `print init_default_if_missing("tc09_init07.txt", "boot=1")`
- **Expected Stdout**: `boot=1`
```pir
f init_default_if_missing (file_name fallback_txt) [
  ? (! (@ex file_name)) [
    @wt file_name fallback_txt
  ]
  r (@rd file_name)
]
```

### TC-09-08: Safe Division Result IsOk
- **Category**: `CAT-09`
- **Prompt**: `Write safe_div_is_ok(num_a, num_b) returning isOk(err("zero")) on 0 divisor else isOk(ok(num_a / num_b)).`
- **Signature Hint**: `f safe_div_is_ok (num_a num_b) [`
- **Test Call (pIR)**: `p (safe_div_is_ok 20 0)`
- **Test Call (.psudo)**: `print safe_div_is_ok(20, 0)`
- **Expected Stdout**: `false`
```pir
f safe_div_is_ok (num_a num_b) [
  ? (== num_b 0) [
    r (isOk (err "zero"))
  ]
  r (isOk (ok (/ num_a num_b)))
]
```

### TC-09-09: Safe Division Unwrap
- **Category**: `CAT-09`
- **Prompt**: `Write safe_div_unwrap(num_a, num_b) returning -1 when num_b == 0 else unwrap(ok(num_a / num_b)).`
- **Signature Hint**: `f safe_div_unwrap (num_a num_b) [`
- **Test Call (pIR)**: `p (safe_div_unwrap 45 9)`
- **Test Call (.psudo)**: `print safe_div_unwrap(45, 9)`
- **Expected Stdout**: `5`
```pir
f safe_div_unwrap (num_a num_b) [
  ? (== num_b 0) [
    r -1
  ]
  r (unwrap (ok (/ num_a num_b)))
]
```

### TC-09-10: Check Negative Error Monad
- **Category**: `CAT-09`
- **Prompt**: `Write check_negative_err(val_x) returning isError(err("neg")) when val_x < 0 else isError(ok(val_x)).`
- **Signature Hint**: `f check_negative_err (val_x) [`
- **Test Call (pIR)**: `p (check_negative_err -3)`
- **Test Call (.psudo)**: `print check_negative_err(-3)`
- **Expected Stdout**: `true`
```pir
f check_negative_err (val_x) [
  ? (< val_x 0) [
    r (isError (err "neg"))
  ]
  r (isError (ok val_x))
]
```


## CAT-10: Stateful OOP Classes (c/m) & Multi-Function Pipelines

### TC-10-01: StepCounter Class
- **Category**: `CAT-10`
- **Prompt**: `Define class StepCounter with method next_val(inst, curr, step_sz) returning curr + step_sz.`
- **Signature Hint**: `c StepCounter [`
- **Test Call (pIR)**: `v cnt (StepCounter)\np (m cnt next_val (10 5))`
- **Test Call (.psudo)**: `let cnt = StepCounter()\nprint cnt.next_val(10, 5)`
- **Expected Stdout**: `15`
```pir
c StepCounter [
  f next_val (inst curr step_sz) [
    r (+ curr step_sz)
  ]
]
```

### TC-10-02: Wallet Class Transaction Guard
- **Category**: `CAT-10`
- **Prompt**: `Create class Wallet with method apply_tx(inst, bal, delta) preventing negative balance.`
- **Signature Hint**: `c Wallet [`
- **Test Call (pIR)**: `v wal (Wallet)\np (m wal apply_tx (100 -35))`
- **Test Call (.psudo)**: `let wal = Wallet()\nprint wal.apply_tx(100, -35)`
- **Expected Stdout**: `65`
```pir
c Wallet [
  f apply_tx (inst bal delta) [
    ? (< (+ bal delta) 0) [
      r bal
    ]
    r (+ bal delta)
  ]
]
```

### TC-10-03: Thermostat Class Action Code
- **Category**: `CAT-10`
- **Prompt**: `Build class Thermostat with action_code(inst, curr_t, target_t) returning 1 to heat, -1 to cool, 0 to hold.`
- **Signature Hint**: `c Thermostat [`
- **Test Call (pIR)**: `v th (Thermostat)\np (m th action_code (18 22))`
- **Test Call (.psudo)**: `let th = Thermostat()\nprint th.action_code(18, 22)`
- **Expected Stdout**: `1`
```pir
c Thermostat [
  f action_code (inst curr_t target_t) [
    ? (< curr_t target_t) [
      r 1
    ]
    ? (> curr_t target_t) [
      r -1
    ]
    r 0
  ]
]
```

### TC-10-04: Box2D Class Area
- **Category**: `CAT-10`
- **Prompt**: `Define class Box2D with method area(inst, width, height) returning width * height.`
- **Signature Hint**: `c Box2D [`
- **Test Call (pIR)**: `v bx (Box2D)\np (m bx area (9 7))`
- **Test Call (.psudo)**: `let bx = Box2D()\nprint bx.area(9, 7)`
- **Expected Stdout**: `63`
```pir
c Box2D [
  f area (inst width height) [
    r (* width height)
  ]
]
```

### TC-10-05: QuotaGate Class Remaining Quota
- **Category**: `CAT-10`
- **Prompt**: `Create class QuotaGate with method remaining(inst, used, limit_q) returning remaining quota.`
- **Signature Hint**: `c QuotaGate [`
- **Test Call (pIR)**: `v qg (QuotaGate)\np (m qg remaining (18 50))`
- **Test Call (.psudo)**: `let qg = QuotaGate()\nprint qg.remaining(18, 50)`
- **Expected Stdout**: `32`
```pir
c QuotaGate [
  f remaining (inst used limit_q) [
    ? (>= used limit_q) [
      r 0
    ]
    r (- limit_q used)
  ]
]
```

### TC-10-06: Sum of Squares Pipeline
- **Category**: `CAT-10`
- **Prompt**: `Write helper sq_val(num_x) and sum_of_squares(pa, pb) calling sq_val on both inputs.`
- **Signature Hint**: `f sq_val (num_x) [`
- **Test Call (pIR)**: `p (sum_of_squares 3 4)`
- **Test Call (.psudo)**: `print sum_of_squares(3, 4)`
- **Expected Stdout**: `25`
```pir
f sq_val (num_x) [
  r (* num_x num_x)
]
f sum_of_squares (pa pb) [
  r (+ (sq_val pa) (sq_val pb))
]
```

### TC-10-07: Checkout Tax Pipeline
- **Category**: `CAT-10`
- **Prompt**: `Write pipeline functions after_discount(amt, disc) and checkout_with_tax(amt, disc, tax).`
- **Signature Hint**: `f after_discount (amt disc) [`
- **Test Call (pIR)**: `p (checkout_with_tax 200 30 17)`
- **Test Call (.psudo)**: `print checkout_with_tax(200, 30, 17)`
- **Expected Stdout**: `187`
```pir
f after_discount (amt disc) [
  r (- amt disc)
]
f checkout_with_tax (amt disc tax) [
  r (+ (after_discount amt disc) tax)
]
```

### TC-10-08: 2D Squared Euclidean Distance
- **Category**: `CAT-10`
- **Prompt**: `Implement diff_sq(x1, x2) and dist_squared_2d(x1, y1, x2, y2) for 2D squared Euclidean distance.`
- **Signature Hint**: `f diff_sq (x1 x2) [`
- **Test Call (pIR)**: `p (dist_squared_2d 1 2 4 6)`
- **Test Call (.psudo)**: `print dist_squared_2d(1, 2, 4, 6)`
- **Expected Stdout**: `25`
```pir
f diff_sq (x1 x2) [
  v d (- x2 x1)
  r (* d d)
]
f dist_squared_2d (x1 y1 x2 y2) [
  r (+ (diff_sq x1 x2) (diff_sq y1 y2))
]
```

### TC-10-09: 2D Point Box Containment
- **Category**: `CAT-10`
- **Prompt**: `Write in_span(val, lo, hi) and point_in_box(px, py, x1, y1, x2, y2) testing 2D box containment.`
- **Signature Hint**: `f in_span (val lo hi) [`
- **Test Call (pIR)**: `p (point_in_box 5 7 0 0 10 10)`
- **Test Call (.psudo)**: `print point_in_box(5, 7, 0, 0, 10, 10)`
- **Expected Stdout**: `true`
```pir
f in_span (val lo hi) [
  ? (< val lo) [
    r false
  ]
  r (<= val hi)
]
f point_in_box (px py x1 y1 x2 y2) [
  ? (in_span px x1 x2) [
    r (in_span py y1 y2)
  ]
  r false
]
```

### TC-10-10: Finite State Machine Runner
- **Category**: `CAT-10`
- **Prompt**: `Write FSM transition helper next_fsm_state(st, evt) and runner run_fsm_three(s0, e1, e2, e3) advancing three events.`
- **Signature Hint**: `f next_fsm_state (st evt) [`
- **Test Call (pIR)**: `p (run_fsm_three 0 1 2 1)`
- **Test Call (.psudo)**: `print run_fsm_three(0, 1, 2, 1)`
- **Expected Stdout**: `3`
```pir
f next_fsm_state (st evt) [
  ? (> evt 0) [
    r (+ st 1)
  ]
  r st
]
f run_fsm_three (s0 e1 e2 e3) [
  v s1 (next_fsm_state s0 e1)
  v s2 (next_fsm_state s1 e2)
  r (next_fsm_state s2 e3)
]
```
