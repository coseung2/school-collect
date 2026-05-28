#!/usr/bin/env python3
"""
OCR 데이터 검증 보조 도구 — 사용자 수정 반영 자동화

사용 시나리오:
    
    1. 선생님이 수정사항을 보내면:
       "단예준 137.7, 유예준 시력 0.1 1.2, 최미진 시력 1.0 0.8"
    
    2. update_csv로 하나씩 반영:
       python update_csv.py ocr_결과.csv "단예준 137.7"
       python update_csv.py ocr_결과.csv "유예준 시력 0.1 1.2"
       python update_csv.py ocr_결과.csv "최미진 시력 1.0 0.8"
    
    3. 또는 한 번에 모두 반영:
       python batch_update.py ocr_결과.csv "수정내용.txt"
    
   수정내용.txt 형식 (한 줄에 하나):
       단예준 137.7
       유예준 시력 0.1 1.2
       최미진 시력 1.0 0.8
       서현우 키 143.2
"""

import subprocess
import sys
import os

def batch_update(csv_path, commands_file):
    if not os.path.exists(commands_file):
        print(f"오류: 파일 없음 → {commands_file}")
        return False
    
    with open(commands_file, encoding='utf-8') as f:
        lines = [l.strip() for l in f if l.strip() and not l.startswith('#')]
    
    success = 0
    fail = 0
    for line in lines:
        print(f"\n>>> {line}")
        result = subprocess.run(
            [sys.executable, 'update_csv.py', csv_path, line],
            capture_output=True, text=True, cwd=os.path.dirname(__file__) or '.'
        )
        print(result.stdout)
        if result.returncode != 0:
            print(result.stderr)
            fail += 1
        else:
            success += 1
    
    print(f"\n{'='*40}")
    print(f"완료: {success}건 성공, {fail}건 실패")
    return fail == 0

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    batch_update(sys.argv[1], sys.argv[2])
