;; Staking Contract
;; Vulnerabilities: reentrancy, underflow, front-running risk

(define-map stakes principal {
    amount: uint,
    start-block: uint,
    rewards-claimed: uint
})
(define-data-var total-staked uint u0)
(define-data-var reward-rate uint u10)
(define-data-var lock-period uint u100)

;; VULNERABLE: unstake sends tokens before clearing stake
(define-public (unstake)
    (let (
        (stake (unwrap! (map-get? stakes tx-sender) (err u1)))
        (amount (get amount stake))
        (start (get start-block stake))
    )
        (asserts! (>= block-height (+ start (var-get lock-period))) (err u2))
        (contract-call? .token transfer amount (as-contract tx-sender) tx-sender)
        (map-delete stakes tx-sender)
        (var-set total-staked (- (var-get total-staked) amount))
        (ok true)))

;; VULNERABLE: claim rewards before updating claimed amount
(define-public (claim-rewards)
    (let (
        (stake (unwrap! (map-get? stakes tx-sender) (err u1)))
        (amount (get amount stake))
        (claimed (get rewards-claimed stake))
        (reward (* amount (var-get reward-rate)))
    )
        (contract-call? .token transfer reward (as-contract tx-sender) tx-sender)
        (map-set stakes tx-sender (merge stake { rewards-claimed: (+ claimed reward) }))
        (ok true)))

;; VULNERABLE: no access control on reward rate
(define-public (set-reward-rate (rate uint))
    (begin
        (var-set reward-rate rate)
        (ok true)))

;; VULNERABLE: integer underflow on unstake amount
(define-public (partial-unstake (amount uint))
    (let (
        (stake (unwrap! (map-get? stakes tx-sender) (err u1)))
        (staked-amount (get amount stake))
    )
        (map-set stakes tx-sender (merge stake {
            amount: (- staked-amount amount)
        }))
        (contract-call? .token transfer amount (as-contract tx-sender) tx-sender)
        (var-set total-staked (- (var-get total-staked) amount))
        (ok true)))

;; SAFE: stake correctly updates state before external interaction
(define-public (stake (amount uint))
    (begin
        (asserts! (is-eq tx-sender contract-caller) (err u3))
        (asserts! (> amount u0) (err u4))
        (map-set stakes tx-sender {
            amount: amount,
            start-block: block-height,
            rewards-claimed: u0
        })
        (var-set total-staked (+ (var-get total-staked) amount))
        (try! (contract-call? .token transfer amount tx-sender (as-contract tx-sender)))
        (ok true)))