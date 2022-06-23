from http.cookiejar import FileCookieJar
import os
import platform
from tkinter import Tk, filedialog


#打开选择文件夹对话框
#Tk().withdraw()


if platform.system().lower() == 'windows':
    print("windows")
    path_xg = "\\" 

elif platform.system().lower() == 'linux':
    print("linux")
    path_xg = "/"



#获得选择好的文件夹路径
link_sf = filedialog.askdirectory()

link_sf = str(os.path.abspath(link_sf))

print(link_sf)

link_of = filedialog.askdirectory()

link_of = str(os.path.abspath(link_of))

print(link_of)

fjname = "宿命回响"

link_of = link_of+path_xg+fjname

print(link_of)

if not os.path.exists(link_of):
    os.makedirs(link_of)
    print("创建文件夹：",link_of)


for item in os.scandir(link_sf):
    os.link(item,link_of+path_xg+item.name)
    print("创建硬链接：",item," ==> ",link_of+item.name)
