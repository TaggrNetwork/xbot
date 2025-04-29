deploy:
	dfx --identity prod deploy --network ic

logs:
	dfx canister --network ic call --query xbot info '("logs")'

stats:
	dfx canister --network ic call --query xbot info '("stats")'

fixture:
	dfx canister --network ic call xbot fixture

status:
	dfx --identity prod canister --network ic status xbot
