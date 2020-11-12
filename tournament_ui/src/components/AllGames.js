import React from 'react';

import './AllGames.scss';

import Game from './Game';
import Leaderboard from './Leaderboard';

export default function AllGames({ games = {}, assets, leaderboard = [] }) {
    if (Object.keys(games).length === 0) {
        return (
            <div className="AllGames waiting">
                <Leaderboard leaderboard={leaderboard} />
                <h2>
                    There aren&apos;t any games right now, as soon as a player
                    connects they will appear here.
                </h2>
            </div>
        );
    }

    return (
        <div className="AllGames active">
            <Leaderboard leaderboard={leaderboard} />
            <div className="gamesWrapper">
                {Object.keys(games).map(key => {
                    if (games[key]) {
                        return (
                            <Game
                                key={key}
                                width={games[key].width}
                                height={games[key].height}
                                food={games[key].food}
                                entities={games[key].entities}
                                baseTiles={games[key].baseTiles}
                                assets={assets}
                                gameID={key}
                            />
                        );
                    }
                })}
            </div>
        </div>
    );
}
