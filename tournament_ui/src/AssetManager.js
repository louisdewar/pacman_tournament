import { generalError } from './state/action';

export default class AssetManager {
    constructor(cb, dispatch) {
        this.assets = {};
        const assetPromises = [];
        this.maxPacman = 8;
        this.maxGhost = 4;
        // Load pacmen
        for (let i = 0; i < this.maxPacman; i++) {
            assetPromises.push(
                this.loadAsset(
                    'pacman' + i,
                    process.env.PUBLIC_URL + 'assets/pacman' + i + '.png'
                )
            );
        }

        // Load ghosts
        for (let i = 0; i < this.maxGhost; i++) {
            assetPromises.push(
                this.loadAsset(
                    'ghost' + i,
                    process.env.PUBLIC_URL + 'assets/ghost' + i + '.png'
                )
            );
        }

        assetPromises.push(
            this.loadAsset('fruit', process.env.PUBLIC_URL + 'assets/fruit.png')
        );
        assetPromises.push(
            this.loadAsset('power', process.env.PUBLIC_URL + 'assets/power.png')
        );
        Promise.all(assetPromises)
            .then(() => {
                cb(this);
            })
            .catch(err => {
                console.error(err);
                dispatch(
                    generalError(
                        'We couldn\'t load the sprites for the game',
                        'SPRITE_FAILED_LOADING'
                    )
                );
            });
    }

    loadAsset(name, path) {
        return new Promise((res, reject) => {
            const image = new Image();
            image.src = path;
            image.onload = () => {
                this.assets[name] = image;
                res();
            };
            image.onerror = () => {
                reject();
            };
        });
    }

    getAsset(name) {
        return this.assets[name];
    }

    getPacman(variant) {
        return this.assets['pacman' + (variant % this.maxPacman)];
    }

    getGhost(variant) {
        return this.assets['ghost' + (variant % this.maxGhost)];
    }
}
