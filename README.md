# Catalyst smart contracts monorepo

This monorepo contains all Catalyst implementations

Catalyst is an implementation of the Asynchronous Swaps. A design which enables synchronously and asynchronously priced assets using shared liquidity for a large set of assets, supporting both volatile assets and stable assets. The design allows market markers to compete with exiting (volatile and stable coin) on-chain AMMs and (volatile and stable coin) cross-chain AMMs using the same amount of liquidity as a single competitor.

Each implementation is contained within its own folder.

- /evm : Solidity implementation targeting the Ethereum Virtual Machine.
- /rust-common : Contains depreciated code
- /simulator : Simulation of the Catalyst logic.
- /solana : Rust implementation targeting the Solana Virtual Machine.

The EVM implementation is used as a reference implementation.

The following section is best viewed on a MarkDown reader which render $\LaTeX$, like VS code.
# On Asset Pricing:

Let $P(w)$ be a non-increasing function representing the marginal price for a token as a function of the current pool balance, $w$. The value of $\Delta w$ can then be naively calculated by multiplying the starting price by the purchase amount.

$$U = \Delta w \cdot P(w_1) + o(f(\Delta w))$$

As $P(w)$ is a non-increasing function, this naive approach only works for small $\Delta w$, as otherwise selling a large batch has a greater return than selling many small batches:   $w_1 < w_2 \implies P(w_1) ≥ P(w_2)$. The opposite is true when buying. The error of such an implementation increases with as the offered input increases: $\Delta w$. The error can be reduced by subdividing the purchase amount, $\Delta w$.
$$U = \sum_{i=0}^N \frac{\Delta w}{N} \cdot P\left(w_1+ i\frac{\Delta w}{N}\right)+o\left(f\left(\frac{\Delta w}{N}\right)\right)$$
When the number of subdivisions, $N$, is increased the error $o\left(f \left(\frac{\Delta w}{N}\right)\right)$ is reduced. The limit of $N \rightarrow \infty$ is the integral over $P(w_1)$. This gives rise to the core equation of Catalyst.

## The Catalyst Equation

Let $P(w)$ be a non-increasing function. The unit gain when providing $\Delta \alpha = \alpha_2 - \alpha_1$ tokens is computed as

$$U = \int_{\alpha_1}^{\alpha_2} P_\alpha(w) \ dw$$

The user now owns $U$ units, which need to be released from a connected integral. 

$$U = \int_{\beta_2}^{\beta_1} P_\beta(w) \ dw$$

$U$ is known. Solving the integral for $\Delta \beta = \beta_2 - \beta_1$ returns the amount of token $\beta$ returned in exchange for $\Delta \alpha$ tokens of token $\alpha$. Since the value of each integral is just a number, it can be wrapped into a token or sent as a payload to a connected integral on another chain, yielding the desired asynchronous property of the AMM. For swaps within the same pool, both equations can be combined into one:

$$\int_{\alpha_1}^{\alpha_2} P_\alpha(w) \ dw = \int_{\beta_2}^{\beta_1} P_\beta(w) \ dw$$

Where the variables are the limits. When $U$ can be reused between integrals, they are said to communicate and a system of communicating integrals is called a **group**. By using integrals to measure liquidity for each asset within a group, liquidity is measured independently. This gives rise to the important asynchronous property which is required for cross-chain swaps.

### Catalyst's Price

Catalyst defines 2 price curves to serve both demand for volatile tokens and tokens with a stable value.

Volatile: $P(w) = \frac{W_\alpha}{w \cdot \ln(2)}$

Amplification: $P^\theta(w)= \frac{1}{w^\theta} \cdot (1-\theta)$

For amplification, the core equation is slightly modified to adjust the price: $U = \int_{W_\alpha \alpha_1 }^{W_\alpha \alpha_2} P^{\theta}_\alpha(w) \ dw$

## The AMM Equations

Using the Catalyst Equation with the price curves, core mathematical equations can be derived.

**Marginal Price**, If someone were to buy/sell an infinitesimal in the pool. the marginal price is the price they would pay. It can be derived in 3 ways: $\lim_{x_\alpha \to 0} y_\beta/x_\alpha$, $\frac{\mathrm{d}}{\mathrm{d}i_\alpha} solve(Invariant, i_\beta)$, or $\frac{P_\alpha(w)}{P_\beta(w)}$.

**SwapToAndFromUnits**, The full swap equation. Assuming the swap path has $\alpha_t$, $\beta_t$ liquidity and the user provides $x$, the user gets $y_\beta$.

**SwapToUnits**, The first swap of a Catalyst swap. It is completely independent of the state of the second leg of the transaction. Within a group $U$ can be used to transparently purchase any token via *SwapFromUnits*. 

**SwapFromUnits**, The last (and second) leg of a Catalyst swap. It is completely independent of the state of the first leg of the transaction. It requires $U$ which can be acquired by selling any token in the group. 

**Invariant**, A measure used to measure the pool value. Specific to the *invariant* measure, is that it is constant whenever a swap is completed. If a pool implements a swap fee, the measure increases as fees accumulate in the pool. The invariant is not invariant to deposits or withdrawals..

### Volatile Tokens

- Marginal price: $\lim_{x \to 0} y_\beta/x_\alpha = \frac{\beta_t}{\alpha_t}  \frac{W_\alpha}{W_\beta}$

- SwapToAndFromUnits: $y_\beta = \beta_t \cdot \left(1-\left(\frac{\alpha_t+x}{\alpha_t}\right)^{-\frac{W_\alpha}{W_\beta}}\right)$

- SwapToUnits: $U= W_\alpha \cdot \log_2\left(\frac{\alpha_t+x_\alpha}{\alpha_t}\right)$

- SwapFromUnits: $y_\beta = \beta_t \cdot \left(1-2^{-\frac{U}{W_\beta}}\right)$

- Invariant: $K = \prod_{i \in \{\alpha, \beta, \dots\}} i_t^{W_i}$

### Amplified Tokens

- Marginal price: $\lim_{x \to 0} y_\beta/x_\alpha = \frac{\left(\alpha_t W_\alpha\right)^\theta}{\left(\beta_t W_\beta\right)^\theta} \frac{W_\beta}{W_\alpha}$

- SwapToAndFromUnits: $y_\beta=\beta_t \left(1-\left(\frac{(\beta \cdot W_\beta)^{1-\theta}_t - \left(\left(\alpha_t \cdot W_\alpha +x_\alpha \cdot W_\alpha \right)^{1-\theta} - \left(\alpha_t\cdot W_\alpha\right)^{1-\theta}\right) }{\left(\beta_t \cdot W_{\beta}\right)^{1-\theta}}\right)^{\frac{1}{1-\theta}}\right)$

- SwapToUnits: $U=\left((\alpha_t  \cdot W_\alpha + x_\alpha  \cdot W_\alpha)^{1-\theta} - \left(\alpha_t  \cdot W_\alpha \right)^{1-\theta} \right)$

- SwapFromUnits: $y_\beta = \beta_t \cdot \left(1 -\left(\frac{\left(\beta_t \cdot W_\beta\right)^{1-\theta} - U }{\left(\beta_t \cdot W_\beta\right)^{1-\theta}}\right)^{\frac{1}{1-\theta}}\right)$

- Invariant: $K = \sum_{i \in \{\alpha, \beta, \dots\}} i^{1-\theta} W_i^{1-\theta}$

### On Mathematics

**TODO, rewrite**
Approximation to reduce gas cost & swap accuracy for small swaps.

## On deposits and withdrawals.

**TODO, rewrite**

When depositing and withdrawing, the net debt distribution within the pool system, group, must be maintained. Let $\alpha_0, \beta_0, ...$ be a reference to net system assets. Then the outstanding units is $\int_{\alpha_{0}}^{\alpha_{t}}P_{\alpha} \! \left(w\right)d w$ . Keep the outstanding units invariant:

$$
 \begin{equation}
 U[\alpha_0] = \int_{\alpha_{0}}^{\alpha_{t}}P \! \left(w\right)d w = \int_{\alpha_{0} +pt_\alpha}^{\alpha_{t} +tk_\alpha}P \! \left( w\right)d w \end{equation}
$$

Where $P(w)$ is the pricing function, $\alpha_t$ is the current asset balance, $tk_\alpha$ is the asset input amount, $\alpha_0$ is net asset reference, and $pt$ is the net asset reference output amount. Solving the integrals yields the relations:
**TODO, rewrite**

In a pool with a price function such that $\int_0^\tau P(w) \ dw < \infty$  for any $\tau > 0$, debt maintenance is slightly different. Intuitively, this is because when tokens are swapped, the value leaves one and enters another. However, when the curve is tight enough that there is a limited amount of liquidity on one side, the pool can Is *emptied* . If one-sided is empty, the other side shouldn’t own 100%. Examine the debt maintenance equation:

$$
 U[\alpha_0] = \int_{w\alpha_{0}}^{w\alpha_{t}}P^\theta \! \left(w\right)d w = \int_{w\alpha_{0} +wpt_\alpha}^{w\alpha_{t} +wtk_\alpha}P^\theta \! \left( w\right)d w$$

Without loss of generality, assume $\alpha_0 < \alpha_t$, then $U[\alpha_0] > 0$:

$$0< \int_{w\alpha_{0} +pt_\alpha}^{w\alpha_{t} +tk_\alpha}P^\theta \! \left( w\right)d w$$

Then examine $pt_\alpha = -\alpha_0 \implies \alpha_0 - pt_\alpha = 0$

$$0< U[\alpha_0'] = \int_{0}^{w\alpha_{t} +wtk_\alpha}P^\theta \! \left( w\right)d w$$

However, since $0<\int_0^\tau P^\theta(w) \ dw < \infty$ for any $\tau > 0$ we can find $\tau > 0$ such that $U[\alpha_0'] = \int_0^\tau P^\theta(w) \ dw$. The result is that $0 < \tau = w\alpha_t+wtk_\alpha, \ tk_\alpha \neq - \alpha_t$ and tokens are left in the pool.


## Dev dependencies

- ganache-cli
  
  - `pnpm install -g ganache`

- eth-brownie
  
  - via [poetry](https://python-poetry.org)  (`brew install poetry`): `poetry install` in `/`
  - via pip: `pip3 install eth-brownie` (check that your $PATH is properly configured).

- Python dependencies in *./pyproject.toml*. Automatically installed with `poetry install`
  
  - Note: You can set the poetry python version via `poetry env use python3.9` for example.

- Blockchain API
  
  - Default: [alchemy](https://www.alchemy.com), export key to `$ALCHEMY_API_TOKEN`
  
  - Alt: [Infura](https://infura.io), edit *./.brownie/network-config.yaml* with Infura RPC.

# Introduction to Brownie & EVM Smart Contracts

Brownie wraps smart contract development in a neat package. For this repository, any smart contract written in Solidity or Vyper will automatically be compiled and deploy-ready. To deploy, fund an account loaded into Brownie:

- `brownie accounts --help`
  
  - `brownie accounts new <NAME OF ACCOUNT>`
    
    - Example: `brownie accounts new deployment` or `brownie accounts new 0` and provide a privatekey.
  
  - `brownie generate new <NAME OF ACCOUNT>`

- Fund the account generated by brownie. [Kovan faucet](https://github.com/kovan-testnet/faucet), [Rinkeby faucet](https://faucet.rinkeby.io).

Open the brownie dev console:

`brownie console --network <mainnet/kovan/development>`

and load the account:

```
acct = accounts.load('<NAME OF ACCOUNT>')
SC = SmartContractName.deploy(*init_vars, {'from': acct})
SC
```

The smart contract has now been deployed. Deployment scripts can be found in `./scripts/deploy/*`

# Contracts

Contracts are stored in *./contracts*. Contracts compiled by brownie, `brownie compile` are stored in *./build*. Brownie will automatically download compatible solidity and vyper versions for internal usage.

### Solidity

To compile solidity contracts directly (not through Brownie), one has to install:

- Solidity
  
  - via brew: `brew tap ethereum/ethereum` then `brew install solidity`
  - via npm: `pnpm install -g solc` (installs solcjs)
  - [soliditylang.org](https://docs.soliditylang.org/en/latest/installing-solidity.html)

- `pnpm install`

- `solc <path-to-contract> --base-path node_modules`

### Vyper

To compile vyper contracts directly, the correct Vyper version should be installed independently of this project. eth-brownie depends on the newest version of Vyper, which the contracts might not be compatible with.

- Vyper
  - via pip: `pip install vyper==<version>`
  - via docker: [vyper.readthedocs.io](https://vyper.readthedocs.io/en/latest/installing-vyper.html#docker)

# Connecting to devnet

The VPS does not expose **geth**, so to connect one has to execute the commands on the VPS's localhost.

The easiest way to achieve this is via ssh port forwarding. To connect localhost on port 10000 (geth) execute:

> ssh -f -N -L 10000:localhost:10000 root@143.198.168.233 

and to connect localhost on port 10001 (bsc_geth) execute:

> ssh -f -N -L 10001:localhost:10001 root@143.198.168.233

All networking connections sent to port 10000 and 10001 will be sent to the VPS's localhost on the respective ports.

Then add the 2 networks to brownie:

> brownie networks add Ethereum polymerase-geth  host=http://127.0.0.1:10000 chainid=1337

> brownie networks add 'Binance Smart Chain' polymerase-bsc-geth  host=http://127.0.0.10001 chainid=1234