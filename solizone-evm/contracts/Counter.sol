// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Counter {
    uint256 public count;

    event Incremented(uint256 newCount);

    function increment() external {
        count += 1;
        emit Incremented(count);
    }

    function fail() external pure {
        revert("intentional failure");
    }
}