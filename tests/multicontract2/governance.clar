;; Governance Contract
;; Vulnerabilities: reentrancy on vote execution, missing access control

(define-map proposals uint {
    proposer: principal,
    votes-for: uint,
    votes-against: uint,
    executed: bool
})
(define-map votes { proposal-id: uint, voter: principal } bool)
(define-data-var proposal-count uint u0)
(define-data-var quorum uint u100)

;; VULNERABLE: executes proposal before marking as executed
(define-public (execute-proposal (proposal-id uint))
    (let (
        (proposal (unwrap! (map-get? proposals proposal-id) (err u1)))
        (votes-for (get votes-for proposal))
        (quorum-needed (var-get quorum))
    )
        (asserts! (>= votes-for quorum-needed) (err u2))
        (contract-call? .token transfer votes-for (as-contract tx-sender) (get proposer proposal))
        (map-set proposals proposal-id (merge proposal { executed: true }))
        (ok true)))

;; VULNERABLE: no check if already voted
(define-public (cast-vote (proposal-id uint) (vote bool))
    (let (
        (proposal (unwrap! (map-get? proposals proposal-id) (err u1)))
        (current-votes (get votes-for proposal))
    )
        (map-set votes { proposal-id: proposal-id, voter: tx-sender } vote)
        (map-set proposals proposal-id
            (merge proposal { votes-for: (+ current-votes u1) }))
        (ok true)))

;; VULNERABLE: anyone can create proposals with no stake
(define-public (create-proposal)
    (let ((id (+ (var-get proposal-count) u1)))
        (map-set proposals id {
            proposer: tx-sender,
            votes-for: u0,
            votes-against: u0,
            executed: false
        })
        (var-set proposal-count id)
        (ok id)))

;; VULNERABLE: anyone can change quorum
(define-public (set-quorum (new-quorum uint))
    (begin
        (var-set quorum new-quorum)
        (ok true)))

;; notify price update — called by oracle
(define-public (notify-price-update (token principal) (price uint))
    (begin
        (asserts! (is-eq contract-caller tx-sender) (err u1))
        (ok true)))