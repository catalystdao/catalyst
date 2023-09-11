forge compile

# Load global envs
set -o allexport
source ../../.env

# Setup each of the envs

arr=()

for filename in envs/*.env; do
    source $filename
    ./deployCatalyst.vaults.sh &
    arr += ("'$!'")
    sleep 1  # The delay is to ensure the scripts aren't writing / reading at the same time and causing a conflict.
done

for i in "${arr[@]}"; do
    wait "$i"
done

sleep 1

arr=()

for filename in envs/*.env; do
    source $filename
    forge script DeployVaults --sig "setConnections()" --fork-url=$RPC_URL &
    arr += ("'$!'")
    sleep 1  # The delay is to ensure the scripts aren't writing / reading at the same time and causing a conflict.
done

for i in "${arr[@]}"; do
    wait "$i"
done

set +o allexport