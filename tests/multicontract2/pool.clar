;; Liquidity Pool Contract
;; Vulnerabilities: cross-contract reentrancy, integer underflow, unchecked returns

(define-map liquidity-providers principal uint)
(define-map pool-shares principal uint)
(define-data-var total-liquidity uint u0)
(define-data-var pool-paused bool false)

;; VULNERABLE: removes liquidity before updating state
;; cross-contract reentrancy via token contract
(define-public (remove-liquidity (amount uint))
    (let (
        (provider-balance (default-to u0 (map-get? liquidity-providers tx-sender)))
        (share (default-to u0 (map-get? pool-shares tx-sender)))
    )
        (asserts! (>= provider-balance amount) (err u1))
        (contract-call? .token transfer amount (as-contract tx-sender) tx-sender)
        (map-set liquidity-providers tx-sender (- provider-balance amount))
        (map-set pool-shares tx-sender (- share amount))
        (var-set total-liquidity (- (var-get total-liquidity) amount))
        (ok true)))

;; VULNERABLE: flash loan with no state update before external call
(define-public (flash-loan (amount uint) (recipient principal))
    (let ((balance (var-get total-liquidity)))
        (asserts! (>= balance amount) (err u2))
        (contract-call? .token transfer amount (as-contract tx-sender) recipient)
        (var-set total-liquidity (- balance amount))
        (ok true)))

;; VULNERABLE: no access control on pause function
(define-public (set-paused (paused bool))
    (begin
        (var-set pool-paused paused)
        (ok true)))

;; SAFE: adds liquidity correctly
(define-public (add-liquidity (amount uint))
    (begin
        (asserts! (is-eq tx-sender contract-caller) (err u3))
        (asserts! (not (var-get pool-paused)) (err u4))
        (map-set liquidity-providers tx-sender amount)
        (var-set total-liquidity (+ (var-get total-liquidity) amount))
        (try! (contract-call? .token transfer amount tx-sender (as-contract tx-sender)))
        (ok true)))