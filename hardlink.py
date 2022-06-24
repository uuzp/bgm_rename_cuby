from http.cookiejar import FileCookieJar
import os
import platform
import shutil
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

""" link_sf = "D:\\test"
link_of = "D:\\视频库" """

fjname = "宿命回响"

link_of = link_of+path_xg+fjname

print(link_of)

if not os.path.exists(link_of):
    os.makedirs(link_of)
    print("创建文件夹：",link_of)


for item in os.scandir(link_sf):
    s = os.path.splitext(item.name)[-1][1:]
    o_file = link_of+path_xg+item.name
    video = "avi,mp4,flv,mkv"
    sub = "ass,str"
    if s in video:
        print("video")
        os.link(item,o_file)
        print("创建硬链接：",item," ==> ",o_file)
    elif s in sub:
        print("sub")
      #  str_item = str(item)
        
      #  str_item_name = str(item.name)
        
      #  print(link_of+path_xg+str_item_name)
      #  os.system(fz+" "+str_item+" "+link_of+path_xg+str_item_name)
        shutil.copy(item,o_file )

        print("复制字幕：",item," ==> ",o_file)
    else:
        print("no video!")
    
