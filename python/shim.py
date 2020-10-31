import socket
import json
import sys

class Ai:
    def __init__(self, address, port, username):
        print('Connecting to {}:{}'.format(address, port))
        self.sock = socket.create_connection((address, port))
        self.sock.send(bytes(json.dumps({ 'username': username }) + '\n', 'utf-8'))

    def start(self):
        buf = ''

        while True:
            index = buf.find('\n')
            if index != -1:
                msg = buf[0:index]
                buf = buf[index + 1:]
                try:
                    data = json.loads(msg)
                    if 'final_score' in data:
                        print('You died your final score was', data['final_score'])
                        return

                    action = self.choose_action(data['view'])
                    print('Playing action', action, 'on tick', data['tick'])
                    self.sock.send(bytes(json.dumps({ 'tick': data['tick'], 'action': action }) + '\n', 'utf-8'))
                except json.decoder.JSONDecodeError:
                    print('msg `%s` from the server was invalid json' % msg)
            else:
                incoming = self.sock.recv(4096).decode('utf-8')
                buf += incoming

    def choose_action(self, game):
        """ 
            You're code goes here, you must return an action:
            'F' => Forward
            'R' => TurnRight
            'L' => TurnLeft
            'E' => Eat
            'S' => Stay
        """

        print('----' * 3)
        for y in range(4):
            for x in range(3):
                entity = 'M' if game[x][y]['mob'] else 'P' if game[x][y]['player'] else '_'
                print(' {}{} '.format(game[x][y]['base'], entity), end='')
            print('')
        print('----' * 3)

        if game[1][1]['base'] != 'L':
            return 'S'

        return 'F'


if __name__ =='__main__':
    try:
        ip = sys.argv[1]
        port = sys.argv[2]
        username = sys.argv[3]
    except IndexError:
        print("Usage: python3 shim.py [ip] [port] [username]")
    ai = Ai(ip, port, username)

    ai.start()
