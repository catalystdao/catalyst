forge compile

# Load global envs
set -o allexport
source ../../.env

# Setup each of the envs
source .mumbai.env
./deployCatalyst.base.sh &
mumbai=$!
sleep 1  # The delay is to ensure the scripts aren't writing / reading at the same time and causing a conflict.

source .sepolia.env
./deployCatalyst.base.sh &
sepolia=$!
sleep 1 # The delay is to ensure the scripts aren't writing / reading at the same time and causing a conflict.

source .base-goerli.env
./deployCatalyst.base.sh &
base_goerli=$!
sleep 1 # The delay is to ensure the scripts aren't writing / reading at the same time and causing a conflict.

wait $base_goerli $mumbai $sepolia

sleep 1

# Set the connections
source .base-goerli.env
forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
base_goerli=$!

source .sepolia.env
forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
sepolia=$!

source .mumbai.env
forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL --broadcast &
mumbai=$!

wait $base_goerli $mumbai $sepolia

set +o allexport