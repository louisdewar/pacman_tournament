import React from 'react';

import './Leaderboard.scss';

export default function Leaderboard({ leaderboard }) {
    return (
        <div className="Leaderboard">
            <h1>Leaderboard:</h1>
            <ul>
                {leaderboard.map((player, i) => {
                    return (
                        <li key={i + player.username}>
                            {i + 1}. {player.username} ({player.highScore})
                        </li>
                    );
                })}
            </ul>
        </div>
    );
}
