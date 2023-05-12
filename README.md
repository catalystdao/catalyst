# Catalyst smart contracts monorepo

This monorepo contains all Catalyst implementations

Catalyst is an implementation of the `Unit of Liquidity` AMM design. A design which used independent pricing to asynchronously price assets using shared liquidity, supporting both volatile assets and stable assets.

Each implementation is contained within its own folder.

- /evm : Solidity implementation targeting the Ethereum Virtual Machine.
- /rust-common : Contains depreciated code
- /simulator : Simulation of the Catalyst logic.
- /solana : Rust implementation targeting the Solana Virtual Machine.

The EVM implementation is used as a reference implementation.

# On Asset Pricing

For an indepth description of how to price assets, read `Unit of Liquidity`. The below seciton contains notable equations.

## The Catalyst Equation

Let $P_i(w)$ be a decreasing, non-negative marginal price function for a token $i$. The equation which describes a Catalyst swap is then defined as:

$$U = \int_{i_t}^{i_t + \Delta i} P_i(w) \ dw$$

Where $i_t$ is the current balance in the vault, $\Delta i$ is the change in balance caused by the user and $U$ is Units: A measure of the value change by the user. The equation can be used both ways, where a positive change implies a "swap in" and a negative change implies a "swap out". It is implies that when assets are swapped out, $U$ the sign is flipped from positive to negative. 

This implies that the full swap from a token $i$ to another token $j$ can be computed as:

$$\int_{i_t}^{i_t + \Delta i} P_i(w) \ dw =- \int_{j_t}^{j_t + \Delta j} P_j(w) \ dw = \int_{j_t + \Delta j}^{j_t} P_j(w) \ dw$$

Notice that even though the full swap is written as a single equation, it can be evaluated in 2 independent slices (based on the previous equation).

### Catalyst's Price

Catalyst defines 2 price curves to serve both demand for volatile tokens and tokens with a stable value.

Volatile: $P(w) = \frac{W_i}{w}$

Amplification: $P^\theta(w)= \frac{W_i}{(W_i \cdot w)^\theta} \cdot (1-\theta)$

## AMM Terms

**Marginal Price**: If someone were to buy/sell an infinitesimal in the vault. the marginal price is the price they would pay. The marginal price can generally be derived in 3 ways: $\lim_{x_\alpha \to 0} y_\beta/x_\alpha$, $\frac{\mathrm{d}}{\mathrm{d}i_\alpha} solve(Invariant, i_\beta)$, or $\frac{P_\alpha(w)}{P_\beta(w)}$.

**sendAsset**: The first swap of a Catalyst swap. It is independent of the state of the second leg of the transaction. Within a vault $U$ can be used to transparently purchase any token via *receiveAsset*. 

**receiveAsset**: The last (and second) leg of a Catalyst swap. It is completely independent of the state of the first leg of the transaction. It requires $U$ which can be acquired by selling any token in the group. 

**LocalSwap**: A combination of *sendAsset* and *receiveAsset* executed atomically, often on a single chain.

**Invariant**: A measure used to measure the vault value. Specific to the *invariant* measure, is that it is constant whenever a swap is completed. If a vault implements a swap fee, the measure increases as fees accumulate in the vault. The invariant is not invariant to deposits or withdrawals.

## The AMM Equations

Using the Catalyst Equation with the price curves, the mathematical swap equations can be derived.

### Volatile Tokens

- Marginal price: $\lim_{x \to 0} y_j/x_i = \frac{j}{i} \frac{W_i}{W_j}$

- SwapToUnits: $U = W_i \cdot \log\left(\frac{i_t+x_i}{i_t}\right)$

- SwapFromUnits: $y_j = j_t \cdot \left(1-\exp\left(-\frac{U}{W_j}\right)\right)$

- Invariant: $K = \prod_{i} i_t^{W_i}$

### Amplified Tokens

- Marginal price: $\lim_{x \to 0} y_j/x_i = \frac{\left(i_t W_i\right)^\theta}{\left(j_t W_j\right)^\theta} \frac{W_j}{W_i}$

- SwapToUnits: $U = \left((i_t  \cdot W_i + x_i  \cdot W_i)^{1-\theta} - \left(i_t  \cdot W_i \right)^{1-\theta} \right)$

- SwapFromUnits: $y_j = j_t \cdot \left(1 -\left(\frac{\left(j_t \cdot W_j\right)^{1-\theta} - U }{\left(j_t \cdot W_j\right)^{1-\theta}}\right)^{\frac{1}{1-\theta}}\right)$

- Invariant: $K = \sum_{i \in \{\alpha, \beta, \dots\}} i^{1-\theta} W_i^{1-\theta}$
