# ProtoHackers

This repository contains solutions to [ProtoHackers](https://protohackers.com/) challenges

## How to build

Use `cargo` to build the Rust solution by providing the desired solution as following:

```bash
# Example to build solution 0: Smoke Test
cargo build --release --features s0
```

Where features can be the implemented solution number: s0 or s1

## Deploy to [Fly.io](https://fly.io/) 

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Signup - it redirects to the browser
fly auth signup

# Create a basic app
# Takes the local Dockerfile and builds it
fly launch

# Deploy to the cloud
fly deploy --local-only
```

## Contributing

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

[GPL3](https://github.com/dorublanzeanu/protohackers/blob/develop/LICENSE)